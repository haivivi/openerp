use std::sync::Arc;

use openerp_core::{ListResult, ServiceError};
use openerp_sql::{Row, SQLStore, Value};

use crate::model::{Task, TaskListQuery, TaskStatus};

/// SQL schema for the tasks table.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS tasks (
    id          TEXT PRIMARY KEY,
    data        TEXT NOT NULL,
    type        TEXT NOT NULL,
    status      TEXT NOT NULL,
    created_by  TEXT,
    create_at   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_task_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_task_type ON tasks(type);
CREATE INDEX IF NOT EXISTS idx_task_create_at ON tasks(create_at);
";

/// Persistent storage for tasks, backed by SQLStore (SQLite).
pub struct TaskStore {
    db: Arc<dyn SQLStore>,
}

impl TaskStore {
    /// Create a new TaskStore and initialise the schema.
    pub fn new(db: Arc<dyn SQLStore>) -> Result<Self, ServiceError> {
        db.exec(SCHEMA, &[])
            .map_err(|e| ServiceError::Storage(format!("task schema init: {e}")))?;
        Ok(Self { db })
    }

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Insert a new task.
    pub fn create(&self, task: &Task) -> Result<(), ServiceError> {
        let data =
            serde_json::to_string(task).map_err(|e| ServiceError::Internal(e.to_string()))?;

        self.db
            .exec(
                "INSERT INTO tasks (id, data, type, status, created_by, create_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                &[
                    Value::Text(task.id.clone()),
                    Value::Text(data),
                    Value::Text(task.task_type.clone()),
                    Value::Text(task.status.as_str().to_string()),
                    match &task.created_by {
                        Some(s) => Value::Text(s.clone()),
                        None => Value::Null,
                    },
                    Value::Text(task.create_at.clone()),
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
                "SELECT data FROM tasks WHERE id = ?1",
                &[Value::Text(id.to_string())],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let row = rows
            .first()
            .ok_or_else(|| ServiceError::NotFound(format!("task {id}")))?;

        row_to_task(row)
    }

    /// Update a task (full replacement of the data column + indexed columns).
    pub fn update(&self, task: &Task) -> Result<(), ServiceError> {
        let data =
            serde_json::to_string(task).map_err(|e| ServiceError::Internal(e.to_string()))?;

        let affected = self
            .db
            .exec(
                "UPDATE tasks SET data = ?1, status = ?2 WHERE id = ?3",
                &[
                    Value::Text(data),
                    Value::Text(task.status.as_str().to_string()),
                    Value::Text(task.id.clone()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::NotFound(format!("task {}", task.id)));
        }
        Ok(())
    }

    /// Delete a task by ID.
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
        Ok(())
    }

    // -----------------------------------------------------------------------
    // List / Query
    // -----------------------------------------------------------------------

    /// List tasks with optional filters.
    pub fn list(&self, query: &TaskListQuery) -> Result<ListResult<Task>, ServiceError> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let mut where_clauses: Vec<String> = Vec::new();
        let mut params: Vec<Value> = Vec::new();
        let mut idx = 1;

        if let Some(ref t) = query.task_type {
            where_clauses.push(format!("type = ?{idx}"));
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

        // Count total
        let count_sql = format!("SELECT COUNT(*) as cnt FROM tasks {where_sql}");
        let count_rows = self
            .db
            .query(&count_sql, &params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let total = count_rows
            .first()
            .and_then(|r| r.get_i64("cnt"))
            .unwrap_or(0) as usize;

        // Fetch page
        let select_sql = format!(
            "SELECT data FROM tasks {where_sql} ORDER BY create_at DESC LIMIT ?{idx} OFFSET ?{}",
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

    // -----------------------------------------------------------------------
    // Engine helpers
    // -----------------------------------------------------------------------

    /// Count RUNNING tasks of a given type.
    pub fn count_running(&self, task_type: &str) -> Result<u32, ServiceError> {
        let rows = self
            .db
            .query(
                "SELECT COUNT(*) as cnt FROM tasks WHERE type = ?1 AND status = ?2",
                &[
                    Value::Text(task_type.to_string()),
                    Value::Text(TaskStatus::Running.as_str().to_string()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(rows
            .first()
            .and_then(|r| r.get_i64("cnt"))
            .unwrap_or(0) as u32)
    }

    /// Fetch oldest PENDING tasks of a given type, ordered by create_at ASC.
    pub fn pending_tasks(&self, task_type: &str, limit: u32) -> Result<Vec<Task>, ServiceError> {
        let rows = self
            .db
            .query(
                "SELECT data FROM tasks WHERE type = ?1 AND status = ?2 \
                 ORDER BY create_at ASC LIMIT ?3",
                &[
                    Value::Text(task_type.to_string()),
                    Value::Text(TaskStatus::Pending.as_str().to_string()),
                    Value::Integer(limit as i64),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        rows.iter().map(row_to_task).collect()
    }

    /// Atomically claim a PENDING task by transitioning it to RUNNING.
    ///
    /// Returns `true` if the task was claimed (status was PENDING and is now RUNNING).
    /// Returns `false` if someone else already claimed it (no rows affected).
    /// This is the CAS (compare-and-swap) that prevents duplicate dispatch.
    pub fn claim_task(&self, task: &Task) -> Result<bool, ServiceError> {
        let data =
            serde_json::to_string(task).map_err(|e| ServiceError::Internal(e.to_string()))?;

        let affected = self
            .db
            .exec(
                "UPDATE tasks SET data = ?1, status = ?2 WHERE id = ?3 AND status = 'PENDING'",
                &[
                    Value::Text(data),
                    Value::Text(task.status.as_str().to_string()),
                    Value::Text(task.id.clone()),
                ],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(affected > 0)
    }

    /// Fetch all RUNNING tasks (for the watchdog).
    pub fn running_tasks(&self) -> Result<Vec<Task>, ServiceError> {
        let rows = self
            .db
            .query(
                "SELECT data FROM tasks WHERE status = ?1",
                &[Value::Text(TaskStatus::Running.as_str().to_string())],
            )
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        rows.iter().map(row_to_task).collect()
    }
}

/// Deserialize a Task from a row's `data` JSON column.
fn row_to_task(row: &Row) -> Result<Task, ServiceError> {
    let json = row
        .get_str("data")
        .ok_or_else(|| ServiceError::Storage("missing data column".into()))?;
    serde_json::from_str(json).map_err(|e| ServiceError::Storage(format!("bad task json: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskStatus;
    use openerp_sql::SqliteStore;

    fn test_store() -> TaskStore {
        let db = Arc::new(SqliteStore::open_in_memory().unwrap());
        TaskStore::new(db).unwrap()
    }

    fn make_task(id: &str, typ: &str, status: TaskStatus) -> Task {
        Task {
            id: id.into(),
            task_type: typ.into(),
            params: serde_json::Value::Null,
            status,
            progress: 0,
            total: 0,
            message: None,
            result: None,
            error: None,
            create_at: openerp_core::now_rfc3339(),
            start_at: None,
            end_at: None,
            timeout: None,
            created_by: None,
        }
    }

    #[test]
    fn create_and_get() {
        let store = test_store();
        let task = make_task("t1", "test.type", TaskStatus::Pending);
        store.create(&task).unwrap();

        let got = store.get("t1").unwrap();
        assert_eq!(got.id, "t1");
        assert_eq!(got.status, TaskStatus::Pending);
    }

    #[test]
    fn update_status() {
        let store = test_store();
        let mut task = make_task("t2", "test.type", TaskStatus::Pending);
        store.create(&task).unwrap();

        task.status = TaskStatus::Running;
        task.start_at = Some(openerp_core::now_rfc3339());
        store.update(&task).unwrap();

        let got = store.get("t2").unwrap();
        assert_eq!(got.status, TaskStatus::Running);
        assert!(got.start_at.is_some());
    }

    #[test]
    fn delete_task() {
        let store = test_store();
        let task = make_task("t3", "test.type", TaskStatus::Completed);
        store.create(&task).unwrap();
        store.delete("t3").unwrap();

        assert!(store.get("t3").is_err());
    }

    #[test]
    fn list_with_filter() {
        let store = test_store();
        store
            .create(&make_task("a1", "type.a", TaskStatus::Pending))
            .unwrap();
        store
            .create(&make_task("a2", "type.a", TaskStatus::Running))
            .unwrap();
        store
            .create(&make_task("b1", "type.b", TaskStatus::Pending))
            .unwrap();

        let result = store
            .list(&TaskListQuery {
                task_type: Some("type.a".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);

        let result = store
            .list(&TaskListQuery {
                status: Some("PENDING".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(result.total, 2);
    }

    #[test]
    fn count_running_and_pending() {
        let store = test_store();
        store
            .create(&make_task("r1", "type.x", TaskStatus::Running))
            .unwrap();
        store
            .create(&make_task("r2", "type.x", TaskStatus::Running))
            .unwrap();
        store
            .create(&make_task("p1", "type.x", TaskStatus::Pending))
            .unwrap();

        assert_eq!(store.count_running("type.x").unwrap(), 2);
        let pending = store.pending_tasks("type.x", 10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "p1");
    }
}
