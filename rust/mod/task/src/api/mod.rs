mod tasks;
mod task_types;

use std::sync::Arc;
use axum::Router;

use crate::engine::TaskEngine;

/// Build the complete task module router.
///
/// Caller-facing routes:
/// - `POST   /tasks`                — create task
/// - `GET    /tasks`                — list tasks
/// - `GET    /tasks/:id`            — get task
/// - `GET    /tasks/:id/@poll`      — long-poll for state change
/// - `POST   /tasks/:id/@cancel`    — cancel task
/// - `DELETE /tasks/:id`            — delete task
/// - `GET    /tasks/:id/@logs`      — read logs
///
/// Executor-facing routes:
/// - `POST   /tasks/:id/@claim`     — claim task (PENDING -> RUNNING)
/// - `POST   /tasks/:id/@progress`  — report progress counters
/// - `POST   /tasks/:id/@heartbeat` — keep alive
/// - `POST   /tasks/:id/@complete`  — mark COMPLETED
/// - `POST   /tasks/:id/@fail`      — mark FAILED
/// - `GET    /tasks/:id/@data`      — load runtime data
/// - `PUT    /tasks/:id/@data`      — save runtime data
/// - `POST   /tasks/:id/@log`       — pipe log lines
///
/// Task-type management:
/// - `GET    /task-types`            — list registered types
/// - `POST   /task-types`           — register type
/// - `DELETE /task-types/:type_key`  — unregister type
pub fn router(engine: Arc<TaskEngine>) -> Router {
    Router::new()
        .merge(tasks::router(Arc::clone(&engine)))
        .merge(task_types::router(engine))
}
