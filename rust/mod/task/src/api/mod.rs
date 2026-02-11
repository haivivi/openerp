mod tasks;
mod task_types;

use std::sync::Arc;
use axum::Router;

use crate::engine::TaskEngine;

/// Build the complete task module router.
///
/// Routes:
/// - `POST   /tasks`              — create task
/// - `GET    /tasks`              — list tasks
/// - `GET    /tasks/:id`          — get task
/// - `GET    /tasks/:id/@poll`    — long-poll
/// - `POST   /tasks/:id/@cancel`  — cancel task
/// - `DELETE /tasks/:id`          — delete task
/// - `GET    /task-types`         — list registered types
/// - `POST   /task-types`         — register type
/// - `DELETE /task-types/:type`   — unregister type
pub fn router(engine: Arc<TaskEngine>) -> Router {
    Router::new()
        .merge(tasks::router(Arc::clone(&engine)))
        .merge(task_types::router(engine))
}
