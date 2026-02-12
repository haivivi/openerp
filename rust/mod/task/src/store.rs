use std::collections::HashMap;
use std::sync::Arc;

use openerp_core::{ListResult, ServiceError};
use openerp_kv::KVStore;
use openerp_sql::{Row, SQLStore, Value};
use openerp_tsdb::{LogEntry, LogQuery, TsDb};

use crate::model::{Task, TaskListQuery, TaskLogEntry, TaskStatus};

// ---------------------------------------------------------------------------
// SQL schema — proper columns, no JSON blob
// ---------------------------------------------------------------------------

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id             TEXT PRIMARY KEY,
    task_type      TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'PENDING',
    total          INTEGER NOT NULL DEFAULT 0,
    success        INTEGER NOT NULL DEFAULT 0,
    failed         INTEGER NOT NULL DEFAULT 0,
    message        TEXT,
    error          TEXT,
    claimed_by     TEXT,
    last_active_at TEXT,
    created_by     TEXT,
    created_at     TEXT NOT NULL,
    started_at     TEXT,
    ended_at       TEXT,
    timeout_secs   INTEGER NOT NULL DEFAULT 3600,
    retry_count    INTEGER NOT NULL DEFAULT 0,
    max_retries    INTEGER NOT NULL DEFAULT 3
);
CREATE INDEX IF NOT EXISTS idx_tasks_status     ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_type       ON tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);
";

const ALL_COLUMNS: &str =
    "id, task_type, status, total, success, failed, message, error, \
     claimed_by, last_active_at, created_by, created_at, started_at, \
     ended_at, timeout_secs, retry_count, max_retries";

// ---------------------------------------------------------------------------
// TaskStore
// ---------------------------------------------------------------------------

/// Persistent storage for tasks.
///
/// - **SQL** — task metadata (all indexed/queryable fields).
/// - **KV**  — executor runtime data (`task:{id}:input`, `task:{id}:data`).
/// - **TSDB** — executor log streams.
pub struct TaskStore {
    db: Arc<dyn SQLStore>,
    kv: Arc<dyn KVStore>,
    ts: Arc<dyn TsDb>,
}

impl TaskStore {
    /// Create a new TaskStore and initialise the SQL schema.
    pub fn new(
        db: Arc<dyn SQLStore>,
        kv: Arc<dyn KVStore>,
        ts: Arc<dyn TsDb>,
    ) -> Result<Self, ServiceError> {
        db.exec(SCHEMA, &[])
            .map_err(|e| ServiceError::Storage(format!("task schema init: {e}")))?;
        Ok(Self { db, kv, ts })
    }

    // =======================================================================
    // SQL — CRUD
    // =======================================================================

    /// Insert a new task.
    pub fn create(&self, task: &Task) -> Result<(), ServiceError> {
        self.db
            .exec(
                &format!(
                    "INSERT INTO tasks ({ALL_COLUMNS}) \
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)"
                ),
                &[
                    Value::Text(task.id.clone()),
                    Value::Text(task.task_type.clone()),
                    Value::Text(task.status.as_str().to_string()),
                    Value::Integer(task.total),
                    Value::Integer(task.success),
                    Value::Integer(task.failed),
                    opt_text(&task.message),
                    opt_text(&task.error),
                    opt_text(&task.claimed_by),
                    opt_text(&task.last_active_at),
                    opt_text(&task.created_by),
                    Value::Text(task.created_at.clone()),
                    opt_text(&task.started_at),
                    opt_text(&task.ended_at),
                    Value::Integer(task.timeout_secs),
                    Value::Integer(task.retry_count),
                    Value::Integer(task.max_retries),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a task by ID.
    pub fn get(&self, id: &str) -> Result<Task, ServiceError> {
        let rows = self
            .db
            .query(
                &format!("SELECT {ALL_COLUMNS} FROM tasks WHERE id = ?1"),
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let row = rows
            .first()
            .ok_or_else(|| ServiceError::NotFound(format!("task {id}")))?;

        row_to_task(row)
    }

    /// Delete a task by ID. Also cleans up KV data.
    pub fn delete(&self, id: &str) -> Result<(), ServiceError> {
        let affected = self
            .db
            .exec(
                "DELETE FROM tasks WHERE id = ?1",
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::NotFound(format!("task {id}")));
        }

        // Best-effort cleanup of KV data.
        let _ = self.kv.delete(&format!("task:{id}:input"));
        let _ = self.kv.delete(&format!("task:{id}:data"));

        Ok(())
    }

    // =======================================================================
    // SQL — List / Query
    // =======================================================================

    /// List tasks with optional filters.
    pub fn list(&self, query: &TaskListQuery) -> Result<ListResult<Task>, ServiceError> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref t) = query.task_type {
            where_clauses.push(format!("task_type = ?{idx}"));
            params.push(Value::Text(t.clone()));
            idx += 1;
        }
        if let Some(ref s) = query.status {
            where_clauses.push(format!("status = ?{idx}"));
            params.push(Value::Text(s.clone()));
            idx += 1;
        }
        if let Some(ref cb) = query.created_by {
            where_clauses.push(format!("created_by = ?{idx}"));
            params.push(Value::Text(cb.clone()));
            idx += 1;
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        // Count
        let count_sql = format!("SELECT COUNT(*) as cnt FROM tasks {where_sql}");
        let count_rows = self
            .db
            .query(&count_sql, &params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let total = count_rows
            .first()
            .and_then(|r| r.get_i64("cnt"))
            .unwrap_or(0) as usize;

        // Page
        let select_sql = format!(
            "SELECT {ALL_COLUMNS} FROM tasks {where_sql} ORDER BY created_at DESC LIMIT ?{idx} OFFSET ?{}",
            idx + 1
        );
        let mut select_params = params;
        select_params.push(Value::Integer(limit as i64));
        select_params.push(Value::Integer(offset as i64));

        let rows = self
            .db
            .query(&select_sql, &select_params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let items = rows
            .iter()
            .map(row_to_task)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ListResult { items, total })
    }

    // =======================================================================
    // SQL — State transitions (used by engine)
    // =======================================================================

    /// Atomically claim a PENDING task: PENDING -> RUNNING (CAS).
    ///
    /// Returns `true` if the task was claimed, `false` if already taken.
    pub fn claim_task(
        &self,
        id: &str,
        claimed_by: &str,
        now: &str,
    ) -> Result<bool, ServiceError> {
        let affected = self
            .db
            .exec(
                "UPDATE tasks SET status = 'RUNNING', claimed_by = ?1, \
                 started_at = ?2, last_active_at = ?2 \
                 WHERE id = ?3 AND status = 'PENDING'",
                &[
                    Value::Text(claimed_by.to_string()),
                    Value::Text(now.to_string()),
                    Value::Text(id.to_string()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        Ok(affected > 0)
    }

    /// Update progress counters. Lightweight: 3 integers + timestamp + optional message.
    pub fn update_progress(
        &self,
        id: &str,
        total: Option<i64>,
        success: Option<i64>,
        failed: Option<i64>,
        message: Option<String>,
        now: &str,
    ) -> Result<(), ServiceError> {
        // Build dynamic SET clause for only the provided fields.
        let mut sets: Vec<String> = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        let mut idx = 1;

        if let Some(v) = total {
            sets.push(format!("total = ?{idx}"));
            params.push(Value::Integer(v));
            idx += 1;
        }
        if let Some(v) = success {
            sets.push(format!("success = ?{idx}"));
            params.push(Value::Integer(v));
            idx += 1;
        }
        if let Some(v) = failed {
            sets.push(format!("failed = ?{idx}"));
            params.push(Value::Integer(v));
            idx += 1;
        }
        if let Some(ref m) = message {
            sets.push(format!("message = ?{idx}"));
            params.push(Value::Text(m.clone()));
            idx += 1;
        }

        // Always update last_active_at.
        sets.push(format!("last_active_at = ?{idx}"));
        params.push(Value::Text(now.to_string()));
        idx += 1;

        params.push(Value::Text(id.to_string()));

        let sql = format!(
            "UPDATE tasks SET {} WHERE id = ?{idx} AND status = 'RUNNING'",
            sets.join(", ")
        );

        let affected = self
            .db
            .exec(&sql, &params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::Validation(format!(
                "task {id} is not in RUNNING state"
            )));
        }
        Ok(())
    }

    /// Heartbeat: refresh last_active_at only.
    pub fn heartbeat(&self, id: &str, now: &str) -> Result<(), ServiceError> {
        let affected = self
            .db
            .exec(
                "UPDATE tasks SET last_active_at = ?1 WHERE id = ?2 AND status = 'RUNNING'",
                &[
                    Value::Text(now.to_string()),
                    Value::Text(id.to_string()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::Validation(format!(
                "task {id} is not in RUNNING state"
            )));
        }
        Ok(())
    }

    /// Mark a RUNNING task as COMPLETED.
    pub fn complete(&self, id: &str, message: Option<&str>, now: &str) -> Result<(), ServiceError> {
        let affected = self
            .db
            .exec(
                "UPDATE tasks SET status = 'COMPLETED', message = ?1, ended_at = ?2 \
                 WHERE id = ?3 AND status = 'RUNNING'",
                &[
                    match message {
                        Some(m) => Value::Text(m.to_string()),
                        None => Value::Null,
                    },
                    Value::Text(now.to_string()),
                    Value::Text(id.to_string()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::Validation(format!(
                "task {id} is not in RUNNING state"
            )));
        }
        Ok(())
    }

    /// Mark a RUNNING task as FAILED.
    pub fn fail(
        &self,
        id: &str,
        error: &str,
        message: Option<&str>,
        now: &str,
    ) -> Result<(), ServiceError> {
        let affected = self
            .db
            .exec(
                "UPDATE tasks SET status = 'FAILED', error = ?1, message = ?2, ended_at = ?3 \
                 WHERE id = ?4 AND status = 'RUNNING'",
                &[
                    Value::Text(error.to_string()),
                    match message {
                        Some(m) => Value::Text(m.to_string()),
                        None => Value::Null,
                    },
                    Value::Text(now.to_string()),
                    Value::Text(id.to_string()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::Validation(format!(
                "task {id} is not in RUNNING state"
            )));
        }
        Ok(())
    }

    /// Cancel a task (PENDING or RUNNING -> CANCELLED).
    pub fn cancel(&self, id: &str, now: &str) -> Result<Task, ServiceError> {
        let task = self.get(id)?;
        match task.status {
            TaskStatus::Pending | TaskStatus::Running => {}
            _ => {
                return Err(ServiceError::Validation(format!(
                    "task {id} is already in terminal state {}",
                    task.status
                )));
            }
        }

        self.db
            .exec(
                "UPDATE tasks SET status = 'CANCELLED', ended_at = ?1 \
                 WHERE id = ?2 AND (status = 'PENDING' OR status = 'RUNNING')",
                &[
                    Value::Text(now.to_string()),
                    Value::Text(id.to_string()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        // Return updated task.
        self.get(id)
    }

    // =======================================================================
    // SQL — Watchdog helpers
    // =======================================================================

    /// Count RUNNING tasks of a given type.
    pub fn count_running(&self, task_type: &str) -> Result<i64, ServiceError> {
        let rows = self
            .db
            .query(
                "SELECT COUNT(*) as cnt FROM tasks WHERE task_type = ?1 AND status = 'RUNNING'",
                &[Value::Text(task_type.to_string())],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(rows.first().and_then(|r| r.get_i64("cnt")).unwrap_or(0))
    }

    /// Fetch RUNNING tasks whose `last_active_at` is older than `threshold_secs` ago.
    pub fn stale_tasks(&self, threshold_secs: i64) -> Result<Vec<Task>, ServiceError> {
        // SQLite string comparison on ISO 8601 / RFC 3339 dates works correctly.
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(threshold_secs);
        let cutoff_str = cutoff.to_rfc3339();

        let rows = self
            .db
            .query(
                &format!(
                    "SELECT {ALL_COLUMNS} FROM tasks \
                     WHERE status = 'RUNNING' AND last_active_at < ?1"
                ),
                &[Value::Text(cutoff_str)],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        rows.iter().map(row_to_task).collect()
    }

    /// Reset a stale RUNNING task back to PENDING, incrementing retry_count.
    /// If retry_count >= max_retries, mark as FAILED instead.
    pub fn reset_stale(&self, id: &str, now: &str) -> Result<(), ServiceError> {
        let task = self.get(id)?;
        if task.status != TaskStatus::Running {
            return Ok(());
        }

        if task.retry_count + 1 >= task.max_retries {
            // Exceeded retries — mark FAILED.
            self.db
                .exec(
                    "UPDATE tasks SET status = 'FAILED', \
                     error = 'max retries exceeded (stale)', \
                     ended_at = ?1, retry_count = retry_count + 1 \
                     WHERE id = ?2 AND status = 'RUNNING'",
                    &[
                        Value::Text(now.to_string()),
                        Value::Text(id.to_string()),
                    ],
                )
                .map_err(|e| ServiceError::Storage(e.to_string()))?;
        } else {
            // Re-queue.
            self.db
                .exec(
                    "UPDATE tasks SET status = 'PENDING', \
                     claimed_by = NULL, started_at = NULL, last_active_at = NULL, \
                     retry_count = retry_count + 1 \
                     WHERE id = ?1 AND status = 'RUNNING'",
                    &[Value::Text(id.to_string())],
                )
                .map_err(|e| ServiceError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// Fetch all RUNNING tasks (for timeout watchdog).
    pub fn running_tasks(&self) -> Result<Vec<Task>, ServiceError> {
        let rows = self
            .db
            .query(
                &format!("SELECT {ALL_COLUMNS} FROM tasks WHERE status = 'RUNNING'"),
                &[],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        rows.iter().map(row_to_task).collect()
    }

    // =======================================================================
    // KV — Input / Runtime data
    // =======================================================================

    /// Store the input parameters for a task (set at creation time).
    pub fn save_input(&self, task_id: &str, input: &[u8]) -> Result<(), ServiceError> {
        self.kv
            .set(&format!("task:{task_id}:input"), input)
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    /// Load the input parameters for a task.
    pub fn load_input(&self, task_id: &str) -> Result<Option<Vec<u8>>, ServiceError> {
        self.kv
            .get(&format!("task:{task_id}:input"))
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    /// Save executor checkpoint / runtime data.
    pub fn save_data(&self, task_id: &str, data: &[u8]) -> Result<(), ServiceError> {
        self.kv
            .set(&format!("task:{task_id}:data"), data)
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    /// Load executor checkpoint / runtime data.
    pub fn load_data(&self, task_id: &str) -> Result<Option<Vec<u8>>, ServiceError> {
        self.kv
            .get(&format!("task:{task_id}:data"))
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    // =======================================================================
    // TSDB — Logs
    // =======================================================================

    /// Append log lines for a task.
    pub fn append_log(
        &self,
        task_id: &str,
        level: &str,
        lines: &[String],
    ) -> Result<(), ServiceError> {
        let stream = format!("task:{task_id}");
        let mut labels = HashMap::new();
        labels.insert("level".to_string(), level.to_string());

        let entries: Vec<LogEntry> = lines
            .iter()
            .map(|line| LogEntry {
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64,
                labels: labels.clone(),
                data: line.as_bytes().to_vec(),
            })
            .collect();

        self.ts
            .write_batch(&stream, entries)
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    /// Query logs for a task.
    pub fn query_logs(
        &self,
        task_id: &str,
        level: Option<&str>,
        limit: usize,
        desc: bool,
    ) -> Result<Vec<TaskLogEntry>, ServiceError> {
        let mut labels = HashMap::new();
        if let Some(l) = level {
            labels.insert("level".to_string(), l.to_string());
        }

        let entries = self
            .ts
            .query(&LogQuery {
                stream: format!("task:{task_id}"),
                labels,
                limit,
                desc,
                start: None,
                end: None,
            })
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(entries
            .into_iter()
            .map(|e| {
                let level = e
                    .labels
                    .get("level")
                    .cloned()
                    .unwrap_or_else(|| "info".to_string());
                let data = String::from_utf8_lossy(&e.data).to_string();
                TaskLogEntry {
                    ts: e.ts,
                    level,
                    data,
                }
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Row -> Task conversion (reads individual columns)
// ---------------------------------------------------------------------------

fn row_to_task(row: &Row) -> Result<Task, ServiceError> {
    let id = row
        .get_str("id")
        .ok_or_else(|| ServiceError::Storage("missing id column".into()))?
        .to_string();

    let task_type = row
        .get_str("task_type")
        .ok_or_else(|| ServiceError::Storage("missing task_type column".into()))?
        .to_string();

    let status_str = row
        .get_str("status")
        .ok_or_else(|| ServiceError::Storage("missing status column".into()))?;
    let status = TaskStatus::from_str(status_str)
        .ok_or_else(|| ServiceError::Storage(format!("invalid status: {status_str}")))?;

    let created_at = row
        .get_str("created_at")
        .ok_or_else(|| ServiceError::Storage("missing created_at column".into()))?
        .to_string();

    Ok(Task {
        id,
        task_type,
        status,
        total: row.get_i64("total").unwrap_or(0),
        success: row.get_i64("success").unwrap_or(0),
        failed: row.get_i64("failed").unwrap_or(0),
        message: row.get_str("message").map(|s| s.to_string()),
        error: row.get_str("error").map(|s| s.to_string()),
        claimed_by: row.get_str("claimed_by").map(|s| s.to_string()),
        last_active_at: row.get_str("last_active_at").map(|s| s.to_string()),
        created_by: row.get_str("created_by").map(|s| s.to_string()),
        created_at,
        started_at: row.get_str("started_at").map(|s| s.to_string()),
        ended_at: row.get_str("ended_at").map(|s| s.to_string()),
        timeout_secs: row.get_i64("timeout_secs").unwrap_or(3600),
        retry_count: row.get_i64("retry_count").unwrap_or(0),
        max_retries: row.get_i64("max_retries").unwrap_or(3),
    })
}

/// Helper to convert Option<String> to Value::Text or Value::Null.
fn opt_text(opt: &Option<String>) -> Value {
    match opt {
        Some(s) => Value::Text(s.clone()),
        None => Value::Null,
    }
}

// Tests are in engine.rs (integration) and will exercise store methods.
