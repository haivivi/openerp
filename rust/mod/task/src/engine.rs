use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

use openerp_core::{new_id, now_rfc3339, ServiceError};

use crate::model::{Task, TaskStatus, TaskType};
use crate::store::TaskStore;

// ---------------------------------------------------------------------------
// Trigger — notification mechanism for executors
// ---------------------------------------------------------------------------

/// Callback fired when a new task is created, so the executor can pick it up.
///
/// Receives the task ID. Implementations should be non-blocking (fire-and-forget).
/// For in-process executors this is a simple closure; for remote executors this
/// could issue an HTTP POST or RPC call inside a `tokio::spawn`.
pub type TriggerFn = Arc<dyn Fn(&str) + Send + Sync>;

// ---------------------------------------------------------------------------
// Type registration (in-memory only)
// ---------------------------------------------------------------------------

struct TypeRegistration {
    type_def: TaskType,
    trigger: Option<TriggerFn>,
}

// ---------------------------------------------------------------------------
// TaskEngine — state machine + notification center
// ---------------------------------------------------------------------------

/// The core task engine.
///
/// This is a **state machine**, not an executor. It:
/// - Records task lifecycle in SQL.
/// - Stores executor runtime data in KV.
/// - Stores executor logs in TSDB.
/// - Notifies executors when new tasks are created (trigger).
/// - Provides APIs for executors to claim, report progress, complete/fail.
/// - Runs a watchdog to detect stale tasks.
pub struct TaskEngine {
    store: Arc<TaskStore>,
    /// Registered task types (in-memory).
    registry: Mutex<HashMap<String, TypeRegistration>>,
    /// Notify waiters when any task state changes (used by long-poll).
    notify: Arc<Notify>,
}

impl TaskEngine {
    /// Create a new engine backed by the given store.
    pub fn new(store: Arc<TaskStore>) -> Self {
        Self {
            store,
            registry: Mutex::new(HashMap::new()),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Access the underlying store.
    pub fn store(&self) -> &Arc<TaskStore> {
        &self.store
    }

    /// Get the Notify handle (for long-poll).
    pub fn notify(&self) -> &Arc<Notify> {
        &self.notify
    }

    // =======================================================================
    // Task type registration
    // =======================================================================

    /// Register a task type with an optional trigger callback.
    pub async fn register(&self, type_def: TaskType, trigger: Option<TriggerFn>) {
        let key = type_def.task_type.clone();
        self.registry
            .lock()
            .await
            .insert(key, TypeRegistration { type_def, trigger });
    }

    /// List all registered task types.
    pub async fn task_types(&self) -> Vec<TaskType> {
        self.registry
            .lock()
            .await
            .values()
            .map(|r| r.type_def.clone())
            .collect()
    }

    /// Unregister a task type.
    pub async fn unregister(&self, task_type: &str) -> bool {
        self.registry.lock().await.remove(task_type).is_some()
    }

    // =======================================================================
    // Task lifecycle — caller-facing
    // =======================================================================

    /// Create a new task. Stores input in KV, fires the trigger, returns the task.
    pub async fn create_task(
        &self,
        task_type: &str,
        input: &serde_json::Value,
        timeout_secs: Option<i64>,
        max_retries: Option<i64>,
        created_by: Option<String>,
    ) -> Result<Task, ServiceError> {
        // Validate type is registered.
        let registry = self.registry.lock().await;
        let reg = registry.get(task_type).ok_or_else(|| {
            ServiceError::Validation(format!("unknown task type: {task_type}"))
        })?;
        let default_timeout = reg.type_def.default_timeout;
        let trigger = reg.trigger.clone();
        drop(registry);

        let task = Task {
            id: new_id(),
            task_type: task_type.to_string(),
            total: 0,
            success: 0,
            failed: 0,
            status: TaskStatus::Pending,
            message: None,
            error: None,
            claimed_by: None,
            last_active_at: None,
            created_by,
            created_at: now_rfc3339(),
            started_at: None,
            ended_at: None,
            timeout_secs: timeout_secs.unwrap_or(if default_timeout > 0 {
                default_timeout
            } else {
                3600
            }),
            retry_count: 0,
            max_retries: max_retries.unwrap_or(3),
        };

        // SQL insert.
        self.store.create(&task)?;

        // Store input in KV.
        let input_bytes = serde_json::to_vec(input)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.store.save_input(&task.id, &input_bytes)?;

        self.notify.notify_waiters();

        // Fire trigger (non-blocking).
        if let Some(trigger) = trigger {
            let task_id = task.id.clone();
            trigger(&task_id);
        }

        Ok(task)
    }

    // =======================================================================
    // Task lifecycle — executor-facing
    // =======================================================================

    /// Claim a task: PENDING -> RUNNING (CAS).
    ///
    /// Returns the updated task if claimed, error if already taken or not found.
    pub fn claim(&self, task_id: &str, claimed_by: &str) -> Result<Task, ServiceError> {
        let now = now_rfc3339();
        let claimed = self.store.claim_task(task_id, claimed_by, &now)?;
        if !claimed {
            let task = self.store.get(task_id)?;
            return Err(ServiceError::Validation(format!(
                "task {} cannot be claimed (status: {})",
                task_id, task.status
            )));
        }
        self.notify.notify_waiters();
        self.store.get(task_id)
    }

    /// Report progress counters.
    pub fn report_progress(
        &self,
        task_id: &str,
        total: Option<i64>,
        success: Option<i64>,
        failed: Option<i64>,
        message: Option<String>,
    ) -> Result<Task, ServiceError> {
        let now = now_rfc3339();
        self.store
            .update_progress(task_id, total, success, failed, message, &now)?;
        self.notify.notify_waiters();
        self.store.get(task_id)
    }

    /// Heartbeat: refresh last_active_at.
    pub fn heartbeat(&self, task_id: &str) -> Result<(), ServiceError> {
        let now = now_rfc3339();
        self.store.heartbeat(task_id, &now)?;
        Ok(())
    }

    /// Mark a task as COMPLETED.
    pub fn complete(
        &self,
        task_id: &str,
        message: Option<&str>,
    ) -> Result<Task, ServiceError> {
        let now = now_rfc3339();
        self.store.complete(task_id, message, &now)?;
        self.notify.notify_waiters();
        self.store.get(task_id)
    }

    /// Mark a task as FAILED.
    pub fn fail(
        &self,
        task_id: &str,
        error: &str,
        message: Option<&str>,
    ) -> Result<Task, ServiceError> {
        let now = now_rfc3339();
        self.store.fail(task_id, error, message, &now)?;
        self.notify.notify_waiters();
        self.store.get(task_id)
    }

    /// Cancel a task (PENDING or RUNNING -> CANCELLED).
    pub fn cancel(&self, task_id: &str) -> Result<Task, ServiceError> {
        let now = now_rfc3339();
        let task = self.store.cancel(task_id, &now)?;
        self.notify.notify_waiters();
        Ok(task)
    }

    // =======================================================================
    // Runtime data (KV passthrough)
    // =======================================================================

    /// Save executor checkpoint data.
    pub fn save_data(&self, task_id: &str, data: &[u8]) -> Result<(), ServiceError> {
        // Also acts as a heartbeat.
        let now = now_rfc3339();
        let _ = self.store.heartbeat(task_id, &now);
        self.store.save_data(task_id, data)
    }

    /// Load executor checkpoint data.
    pub fn load_data(&self, task_id: &str) -> Result<Option<Vec<u8>>, ServiceError> {
        self.store.load_data(task_id)
    }

    /// Load input params.
    pub fn load_input(&self, task_id: &str) -> Result<Option<Vec<u8>>, ServiceError> {
        self.store.load_input(task_id)
    }

    // =======================================================================
    // Query (SQL passthrough)
    // =======================================================================

    /// Get a single task by ID.
    pub fn get(&self, task_id: &str) -> Result<Task, ServiceError> {
        self.store.get(task_id)
    }

    /// List tasks with optional filters.
    pub fn list(&self, query: crate::model::TaskListQuery) -> Result<openerp_core::ListResult<Task>, ServiceError> {
        self.store.list(&query)
    }

    // =======================================================================
    // Task type management (persistent-friendly wrappers)
    // =======================================================================

    /// Register a task type via API (no trigger callback — trigger is code-only).
    pub async fn register_task_type(
        &self,
        req: crate::model::RegisterTaskTypeRequest,
    ) -> Result<TaskType, ServiceError> {
        let tt = TaskType {
            task_type: req.task_type.clone(),
            service: req.service,
            description: req.description,
            default_timeout: req.default_timeout,
            max_concurrency: req.max_concurrency,
        };
        self.register(tt.clone(), None).await;
        Ok(tt)
    }

    /// Get a task type by its key.
    pub async fn get_task_type(&self, task_type: &str) -> Result<TaskType, ServiceError> {
        let reg = self.registry.lock().await;
        reg.get(task_type)
            .map(|r| r.type_def.clone())
            .ok_or_else(|| ServiceError::NotFound(format!("task type {task_type}")))
    }

    /// List all registered task types.
    pub async fn list_task_types(&self) -> Result<Vec<TaskType>, ServiceError> {
        Ok(self.task_types().await)
    }

    /// Delete a task type.
    pub async fn delete_task_type(&self, task_type: &str) -> Result<(), ServiceError> {
        if self.unregister(task_type).await {
            Ok(())
        } else {
            Err(ServiceError::NotFound(format!("task type {task_type}")))
        }
    }

    // =======================================================================
    // Long-poll
    // =======================================================================

    /// Long-poll: wait up to `timeout_secs` for any state change, then return
    /// the current task. Returns immediately if the task is already terminal.
    pub async fn poll(&self, task_id: &str, timeout_secs: u64) -> Result<Task, ServiceError> {
        let task = self.store.get(task_id)?;
        if task.status.is_terminal() {
            return Ok(task);
        }

        let timeout = std::time::Duration::from_secs(timeout_secs.min(120));
        let notified = self.notify.notified();

        match tokio::time::timeout(timeout, notified).await {
            Ok(()) => {
                // Something changed — return fresh state.
                self.store.get(task_id)
            }
            Err(_) => {
                // Timeout — return current state.
                self.store.get(task_id)
            }
        }
    }

    // =======================================================================
    // Logs (TSDB passthrough)
    // =======================================================================

    /// Append log lines for a task.
    pub fn append_log(
        &self,
        task_id: &str,
        level: &str,
        lines: &[String],
    ) -> Result<(), ServiceError> {
        self.store.append_log(task_id, level, lines)
    }

    /// Query logs for a task.
    pub fn query_logs(
        &self,
        task_id: &str,
        limit: Option<usize>,
        desc: bool,
        level: Option<&str>,
    ) -> Result<Vec<crate::model::TaskLogEntry>, ServiceError> {
        self.store.query_logs(task_id, level, limit.unwrap_or(100), desc)
    }

    // =======================================================================
    // Watchdog
    // =======================================================================

    /// Check all RUNNING tasks for timeout. Mark expired ones as FAILED.
    pub fn check_timeouts(&self) -> Result<u32, ServiceError> {
        let running = self.store.running_tasks()?;
        let now = chrono::Utc::now();
        let now_str = now_rfc3339();
        let mut timed_out = 0u32;

        for task in running {
            let timeout_secs = task.timeout_secs;
            if timeout_secs <= 0 {
                continue;
            }

            let started = match &task.started_at {
                Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(_) => continue,
                },
                None => continue,
            };

            let elapsed = (now - started).num_seconds();
            if elapsed >= timeout_secs {
                self.store
                    .fail(&task.id, "timeout", None, &now_str)?;
                self.notify.notify_waiters();
                timed_out += 1;
            }
        }

        Ok(timed_out)
    }

    /// Check for stale RUNNING tasks (no heartbeat) and reset them.
    pub fn check_stale(&self, stale_threshold_secs: i64) -> Result<u32, ServiceError> {
        let stale = self.store.stale_tasks(stale_threshold_secs)?;
        let now = now_rfc3339();
        let mut count = 0u32;

        for task in stale {
            self.store.reset_stale(&task.id, &now)?;
            self.notify.notify_waiters();
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskType;
    use crate::store::TaskStore;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn make_engine() -> Arc<TaskEngine> {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(openerp_sql::SqliteStore::open_in_memory().unwrap());
        let kv = Arc::new(
            openerp_kv::RedbStore::open(&dir.path().join("kv.redb")).unwrap(),
        );
        let ts = Arc::new(
            openerp_tsdb::WalEngine::open(&dir.path().join("tsdb")).unwrap(),
        );
        // Leak dir so it lives long enough for the test.
        let _dir = Box::leak(Box::new(dir));
        let store = Arc::new(TaskStore::new(db, kv, ts).unwrap());
        Arc::new(TaskEngine::new(store))
    }

    fn test_type(name: &str) -> TaskType {
        TaskType {
            task_type: name.into(),
            service: "test".into(),
            description: String::new(),
            default_timeout: 0,
            max_concurrency: 0,
        }
    }

    #[tokio::test]
    async fn create_and_get() {
        let engine = make_engine();
        engine.register(test_type("test.echo"), None).await;

        let task = engine
            .create_task(
                "test.echo",
                &serde_json::json!({"hello": "world"}),
                None,
                None,
                Some("tester".into()),
            )
            .await
            .unwrap();

        assert_eq!(task.task_type, "test.echo");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.created_by.as_deref(), Some("tester"));

        // Input should be stored in KV.
        let input = engine.load_input(&task.id).unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&input).unwrap();
        assert_eq!(parsed, serde_json::json!({"hello": "world"}));
    }

    #[tokio::test]
    async fn create_unknown_type_fails() {
        let engine = make_engine();
        let result = engine
            .create_task("nonexistent", &serde_json::Value::Null, None, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn trigger_fires_on_create() {
        let engine = make_engine();
        let triggered = Arc::new(AtomicBool::new(false));
        let triggered_clone = Arc::clone(&triggered);

        let trigger: TriggerFn = Arc::new(move |_task_id| {
            triggered_clone.store(true, Ordering::SeqCst);
        });

        engine
            .register(test_type("test.trigger"), Some(trigger))
            .await;

        engine
            .create_task("test.trigger", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();

        assert!(triggered.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn claim_and_complete() {
        let engine = make_engine();
        engine.register(test_type("test.work"), None).await;

        let task = engine
            .create_task("test.work", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();

        // Claim.
        let claimed = engine.claim(&task.id, "worker-1").unwrap();
        assert_eq!(claimed.status, TaskStatus::Running);
        assert_eq!(claimed.claimed_by.as_deref(), Some("worker-1"));

        // Double-claim should fail.
        assert!(engine.claim(&task.id, "worker-2").is_err());

        // Report progress.
        let updated = engine
            .report_progress(&task.id, Some(100), Some(50), Some(2), Some("halfway".into()))
            .unwrap();
        assert_eq!(updated.total, 100);
        assert_eq!(updated.success, 50);
        assert_eq!(updated.failed, 2);
        assert_eq!(updated.message.as_deref(), Some("halfway"));

        // Complete.
        let completed = engine.complete(&task.id, Some("all done")).unwrap();
        assert_eq!(completed.status, TaskStatus::Completed);
        assert!(completed.ended_at.is_some());
    }

    #[tokio::test]
    async fn claim_and_fail() {
        let engine = make_engine();
        engine.register(test_type("test.fail"), None).await;

        let task = engine
            .create_task("test.fail", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();

        engine.claim(&task.id, "worker-1").unwrap();
        let failed = engine.fail(&task.id, "something broke", None).unwrap();
        assert_eq!(failed.status, TaskStatus::Failed);
        assert_eq!(failed.error.as_deref(), Some("something broke"));
    }

    #[tokio::test]
    async fn cancel_pending() {
        let engine = make_engine();
        engine.register(test_type("test.cancel"), None).await;

        let task = engine
            .create_task("test.cancel", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();

        let cancelled = engine.cancel(&task.id).unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn cancel_running() {
        let engine = make_engine();
        engine.register(test_type("test.cancel"), None).await;

        let task = engine
            .create_task("test.cancel", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();
        engine.claim(&task.id, "w").unwrap();

        let cancelled = engine.cancel(&task.id).unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn heartbeat_refreshes() {
        let engine = make_engine();
        engine.register(test_type("test.hb"), None).await;

        let task = engine
            .create_task("test.hb", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();
        engine.claim(&task.id, "w").unwrap();

        let before = engine.store().get(&task.id).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        engine.heartbeat(&task.id).unwrap();
        let after = engine.store().get(&task.id).unwrap();

        // last_active_at should have been refreshed.
        assert!(after.last_active_at >= before.last_active_at);
    }

    #[tokio::test]
    async fn runtime_data_roundtrip() {
        let engine = make_engine();
        engine.register(test_type("test.data"), None).await;

        let task = engine
            .create_task("test.data", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();
        engine.claim(&task.id, "w").unwrap();

        // No data initially.
        assert!(engine.load_data(&task.id).unwrap().is_none());

        // Save and load.
        engine
            .save_data(&task.id, b"{\"checkpoint\":42}")
            .unwrap();
        let data = engine.load_data(&task.id).unwrap().unwrap();
        assert_eq!(data, b"{\"checkpoint\":42}");
    }

    #[tokio::test]
    async fn log_roundtrip() {
        let engine = make_engine();
        engine.register(test_type("test.log"), None).await;

        let task = engine
            .create_task("test.log", &serde_json::Value::Null, None, None, None)
            .await
            .unwrap();

        engine
            .append_log(
                &task.id,
                "info",
                &["line 1".to_string(), "line 2".to_string()],
            )
            .unwrap();

        let logs = engine.store().query_logs(&task.id, None, 10, false).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].data, "line 1");
        assert_eq!(logs[1].data, "line 2");
    }
}
