use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

use openerp_core::{new_id, now_rfc3339, ServiceError};

use crate::model::{Task, TaskStatus, TaskType};
use crate::store::TaskStore;

// ---------------------------------------------------------------------------
// TaskContext — passed to handlers for progress reporting + cancellation
// ---------------------------------------------------------------------------

/// Context available to a running task handler.
///
/// Handlers use this to report progress and check for cancellation.
pub struct TaskContext {
    task_id: String,
    store: Arc<TaskStore>,
    notify: Arc<Notify>,
    cancel: CancellationToken,
}

impl TaskContext {
    /// Report progress.  `current` / `total` with an optional human message.
    pub fn report_progress(
        &self,
        current: u64,
        total: u64,
        message: Option<String>,
    ) -> Result<(), ServiceError> {
        let mut task = self.store.get(&self.task_id)?;
        task.progress = current;
        task.total = total;
        task.message = message;
        self.store.update(&task)?;
        self.notify.notify_waiters();
        Ok(())
    }

    /// Returns a CancellationToken that the handler should select on.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// The task ID.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

// ---------------------------------------------------------------------------
// Handler type
// ---------------------------------------------------------------------------

/// The async function signature a handler must satisfy.
///
/// Receives the full Task (for reading params) and a TaskContext (for progress).
/// Returns `Ok(result_json)` on success, `Err` on failure.
pub type TaskHandler = Arc<
    dyn Fn(Task, Arc<TaskContext>) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ServiceError>> + Send>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// TaskEngine
// ---------------------------------------------------------------------------

struct TypeRegistration {
    handler: TaskHandler,
    type_def: TaskType,
}

/// The core task engine: handler registry, submit, cancel, and dispatch.
pub struct TaskEngine {
    store: Arc<TaskStore>,
    /// Registered handlers keyed by task type string.
    registry: Mutex<HashMap<String, TypeRegistration>>,
    /// Per-task cancellation tokens, keyed by task id.
    /// Wrapped in Arc so spawned tasks can clean up after themselves.
    cancellations_shared: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// Notify waiters when any task state changes (used by long-poll).
    notify: Arc<Notify>,
}

impl TaskEngine {
    /// Create a new engine backed by the given store.
    pub fn new(store: Arc<TaskStore>) -> Self {
        Self {
            store,
            registry: Mutex::new(HashMap::new()),
            cancellations_shared: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Access the underlying store (for API layer reads).
    pub fn store(&self) -> &Arc<TaskStore> {
        &self.store
    }

    /// Get the Notify handle (for long-poll).
    pub fn notify(&self) -> &Arc<Notify> {
        &self.notify
    }

    // -----------------------------------------------------------------------
    // Handler registration
    // -----------------------------------------------------------------------

    /// Register a handler for a task type.
    pub async fn register<F, Fut>(&self, type_def: TaskType, handler: F)
    where
        F: Fn(Task, Arc<TaskContext>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<serde_json::Value, ServiceError>> + Send + 'static,
    {
        let key = type_def.task_type.clone();
        let handler: TaskHandler = Arc::new(move |task, ctx| Box::pin(handler(task, ctx)));
        let reg = TypeRegistration { handler, type_def };
        self.registry.lock().await.insert(key, reg);
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

    // -----------------------------------------------------------------------
    // Submit
    // -----------------------------------------------------------------------

    /// Submit a new task.  Returns the created Task (status = PENDING).
    ///
    /// The engine immediately tries to dispatch it if concurrency allows.
    pub async fn submit(
        self: &Arc<Self>,
        task_type: &str,
        params: serde_json::Value,
        timeout: Option<u64>,
        created_by: Option<String>,
    ) -> Result<Task, ServiceError> {
        // Ensure type is registered.
        let registry = self.registry.lock().await;
        if !registry.contains_key(task_type) {
            return Err(ServiceError::Validation(format!(
                "unknown task type: {task_type}"
            )));
        }
        let default_timeout = registry[task_type].type_def.default_timeout;
        drop(registry);

        let task = Task {
            id: new_id(),
            task_type: task_type.to_string(),
            params,
            status: TaskStatus::Pending,
            progress: 0,
            total: 0,
            message: None,
            result: None,
            error: None,
            create_at: now_rfc3339(),
            start_at: None,
            end_at: None,
            timeout: timeout.or(if default_timeout > 0 {
                Some(default_timeout)
            } else {
                None
            }),
            created_by,
        };

        self.store.create(&task)?;
        self.notify.notify_waiters();

        // Try to dispatch immediately.
        let engine = Arc::clone(self);
        let task_type_clone = task.task_type.clone();
        tokio::spawn(async move {
            let _ = engine.try_dispatch(&task_type_clone).await;
        });

        Ok(task)
    }

    // -----------------------------------------------------------------------
    // Cancel
    // -----------------------------------------------------------------------

    /// Cancel a task.  If running, signals the CancellationToken.
    pub async fn cancel(&self, task_id: &str) -> Result<Task, ServiceError> {
        let mut task = self.store.get(task_id)?;

        match task.status {
            TaskStatus::Pending => {
                task.status = TaskStatus::Cancelled;
                task.end_at = Some(now_rfc3339());
                self.store.update(&task)?;
                self.notify.notify_waiters();
            }
            TaskStatus::Running => {
                // Signal the handler's cancellation token.
                if let Some(token) = self.cancellations_shared.lock().await.get(task_id) {
                    token.cancel();
                }
                task.status = TaskStatus::Cancelled;
                task.end_at = Some(now_rfc3339());
                self.store.update(&task)?;
                self.notify.notify_waiters();
            }
            _ => {
                return Err(ServiceError::Validation(format!(
                    "task {} is already in terminal state {}",
                    task_id, task.status
                )));
            }
        }

        // Clean up cancellation token.
        self.cancellations_shared.lock().await.remove(task_id);

        Ok(task)
    }

    // -----------------------------------------------------------------------
    // Dispatch — pick up PENDING tasks and execute
    // -----------------------------------------------------------------------

    /// Try to dispatch pending tasks of the given type, respecting concurrency limits.
    pub async fn try_dispatch(&self, task_type: &str) -> Result<(), ServiceError> {
        let registry = self.registry.lock().await;
        let reg = match registry.get(task_type) {
            Some(r) => r,
            None => return Ok(()),
        };

        let max_concurrency = reg.type_def.max_concurrency;
        let handler = Arc::clone(&reg.handler);
        drop(registry);

        // How many slots are available?
        let running = self.store.count_running(task_type)?;
        let slots = if max_concurrency == 0 {
            // Unlimited — dispatch up to 64 at a time.
            64u32.saturating_sub(running)
        } else {
            max_concurrency.saturating_sub(running)
        };

        if slots == 0 {
            return Ok(());
        }

        let pending = self.store.pending_tasks(task_type, slots)?;

        for mut task in pending {
            // Atomically claim: PENDING → RUNNING (CAS).
            // If another concurrent dispatch already claimed this task,
            // claim_task returns false and we skip it.
            task.status = TaskStatus::Running;
            task.start_at = Some(now_rfc3339());
            let claimed = self.store.claim_task(&task)?;
            if !claimed {
                continue;
            }
            self.notify.notify_waiters();

            // Create cancellation token.
            let cancel = CancellationToken::new();
            self.cancellations_shared
                .lock()
                .await
                .insert(task.id.clone(), cancel.clone());

            let ctx = Arc::new(TaskContext {
                task_id: task.id.clone(),
                store: Arc::clone(&self.store),
                notify: Arc::clone(&self.notify),
                cancel,
            });

            let task_id = task.id.clone();
            let handler = Arc::clone(&handler);
            let engine_notify = Arc::clone(&self.notify);
            let engine_store = Arc::clone(&self.store);
            let engine_cancellations = Arc::clone(&self.cancellations_shared);

            tokio::spawn(async move {
                // Run handler in a nested spawn so we can detect panics via
                // JoinHandle. Without this, a panic aborts the outer future
                // and the task stays RUNNING forever, consuming a concurrency slot.
                let handler_handle = tokio::spawn(
                    (handler)(task.clone(), Arc::clone(&ctx)),
                );
                let result = handler_handle.await;

                // Re-read task in case handler updated progress.
                let mut final_task = match engine_store.get(&task_id) {
                    Ok(t) => t,
                    Err(_) => task,
                };

                // Only update if still RUNNING (might have been cancelled).
                if final_task.status == TaskStatus::Running {
                    match result {
                        Ok(Ok(value)) => {
                            final_task.status = TaskStatus::Completed;
                            final_task.result = Some(value);
                            final_task.end_at = Some(now_rfc3339());
                        }
                        Ok(Err(e)) => {
                            final_task.status = TaskStatus::Failed;
                            final_task.error = Some(e.to_string());
                            final_task.end_at = Some(now_rfc3339());
                        }
                        Err(join_err) => {
                            // Handler panicked or was cancelled.
                            final_task.status = TaskStatus::Failed;
                            final_task.error = Some(format!("handler panicked: {join_err}"));
                            final_task.end_at = Some(now_rfc3339());
                        }
                    }
                    let _ = engine_store.update(&final_task);
                }

                // Clean up cancellation token to prevent memory leak.
                engine_cancellations.lock().await.remove(&final_task.id);

                engine_notify.notify_waiters();
            });
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Watchdog — timeout check
    // -----------------------------------------------------------------------

    /// Check all RUNNING tasks for timeout.  Mark expired ones as FAILED.
    pub async fn check_timeouts(&self) -> Result<u32, ServiceError> {
        let running = self.store.running_tasks()?;
        let now = chrono::Utc::now();
        let mut timed_out = 0u32;

        for mut task in running {
            let timeout_secs = match task.timeout {
                Some(t) if t > 0 => t,
                _ => continue,
            };

            let started = match &task.start_at {
                Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(_) => continue,
                },
                None => continue,
            };

            let elapsed = (now - started).num_seconds();
            if elapsed >= timeout_secs as i64 {
                // Cancel the handler.
                if let Some(token) = self.cancellations_shared.lock().await.remove(&task.id) {
                    token.cancel();
                }
                task.status = TaskStatus::Failed;
                task.error = Some("timeout".into());
                task.end_at = Some(now_rfc3339());
                self.store.update(&task)?;
                self.notify.notify_waiters();
                timed_out += 1;
            }
        }

        Ok(timed_out)
    }

    /// Scan all registered types and try to dispatch any pending work.
    /// Called periodically by the background worker.
    pub async fn dispatch_all(&self) -> Result<(), ServiceError> {
        let types: Vec<String> = self
            .registry
            .lock()
            .await
            .keys()
            .cloned()
            .collect();

        for t in types {
            self.try_dispatch(&t).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskType;
    use crate::store::TaskStore;
    use openerp_sql::SqliteStore;

    fn make_engine() -> Arc<TaskEngine> {
        let db = Arc::new(SqliteStore::open_in_memory().unwrap());
        let store = Arc::new(TaskStore::new(db).unwrap());
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
    async fn register_and_submit() {
        let engine = make_engine();

        engine
            .register(test_type("test.echo"), |task, _ctx| async move {
                Ok(task.params.clone())
            })
            .await;

        let task = engine
            .submit(
                "test.echo",
                serde_json::json!({"hello": "world"}),
                None,
                Some("tester".into()),
            )
            .await
            .unwrap();

        assert_eq!(task.task_type, "test.echo");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.created_by.as_deref(), Some("tester"));

        // Wait a bit for the spawned task to execute.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let updated = engine.store().get(&task.id).unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.result, Some(serde_json::json!({"hello": "world"})));
    }

    #[tokio::test]
    async fn submit_unknown_type_fails() {
        let engine = make_engine();
        let result = engine
            .submit("nonexistent", serde_json::Value::Null, None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cancel_pending_task() {
        let engine = make_engine();

        // Register with max_concurrency = 1 so second task stays PENDING.
        let mut td = test_type("test.slow");
        td.max_concurrency = 1;

        engine
            .register(td, |_task, ctx| async move {
                // Block until cancelled.
                ctx.cancellation_token().cancelled().await;
                Ok(serde_json::Value::Null)
            })
            .await;

        // Submit first (will run), then second (will be pending).
        let _first = engine
            .submit("test.slow", serde_json::Value::Null, None, None)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let second = engine
            .submit("test.slow", serde_json::Value::Null, None, None)
            .await
            .unwrap();
        // Second should be PENDING because max_concurrency=1.
        let second_check = engine.store().get(&second.id).unwrap();
        assert_eq!(second_check.status, TaskStatus::Pending);

        // Cancel it.
        let cancelled = engine.cancel(&second.id).await.unwrap();
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn handler_failure_marks_task_failed() {
        let engine = make_engine();

        engine
            .register(test_type("test.fail"), |_task, _ctx| async move {
                Err(ServiceError::Internal("boom".into()))
            })
            .await;

        let task = engine
            .submit("test.fail", serde_json::Value::Null, None, None)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let updated = engine.store().get(&task.id).unwrap();
        assert_eq!(updated.status, TaskStatus::Failed);
        assert!(updated.error.as_ref().unwrap().contains("boom"));
    }

    #[tokio::test]
    async fn progress_reporting() {
        let engine = make_engine();

        engine
            .register(test_type("test.progress"), |_task, ctx| async move {
                for i in 1..=5 {
                    ctx.report_progress(i, 5, Some(format!("step {i}/5")))
                        .unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                Ok(serde_json::json!({"done": true}))
            })
            .await;

        let task = engine
            .submit("test.progress", serde_json::Value::Null, None, None)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let updated = engine.store().get(&task.id).unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
        assert_eq!(updated.progress, 5);
        assert_eq!(updated.total, 5);
    }

    #[tokio::test]
    async fn concurrency_control() {
        let engine = make_engine();

        let mut td = test_type("test.limited");
        td.max_concurrency = 2;

        let counter = Arc::new(tokio::sync::Semaphore::new(0));
        let counter_clone = Arc::clone(&counter);

        engine
            .register(td, move |_task, _ctx| {
                let c = Arc::clone(&counter_clone);
                async move {
                    c.add_permits(1);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    Ok(serde_json::Value::Null)
                }
            })
            .await;

        // Submit 4 tasks.
        for _ in 0..4 {
            engine
                .submit("test.limited", serde_json::Value::Null, None, None)
                .await
                .unwrap();
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Only 2 should be running.
        let running = engine.store().count_running("test.limited").unwrap();
        assert_eq!(running, 2);
    }
}
