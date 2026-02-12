use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use openerp_core::ServiceError;

use crate::engine::TaskEngine;
use crate::model::{CreateTaskRequest, PollQuery, Task, TaskListQuery};

type EngineState = Arc<TaskEngine>;

pub fn router(engine: Arc<TaskEngine>) -> Router {
    Router::new()
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/{id}", get(get_task).delete(delete_task))
        .route("/tasks/{id}/@poll", get(poll_task))
        .route("/tasks/{id}/@cancel", post(cancel_task))
        .with_state(engine)
}

// ---------------------------------------------------------------------------
// POST /tasks
// ---------------------------------------------------------------------------

async fn create_task(
    State(engine): State<EngineState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine
        .submit(&req.task_type, req.params, req.timeout, req.created_by)
        .await?;
    Ok(Json(task))
}

// ---------------------------------------------------------------------------
// GET /tasks
// ---------------------------------------------------------------------------

async fn list_tasks(
    State(engine): State<EngineState>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = engine.store().list(&query)?;
    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
    })))
}

// ---------------------------------------------------------------------------
// GET /tasks/:id
// ---------------------------------------------------------------------------

async fn get_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.store().get(&id)?;
    Ok(Json(task))
}

// ---------------------------------------------------------------------------
// GET /tasks/:id/@poll
// ---------------------------------------------------------------------------

async fn poll_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Query(query): Query<PollQuery>,
) -> Result<Json<Task>, ServiceError> {
    let timeout = Duration::from_secs(query.timeout.min(120));
    let notify = engine.notify().clone();
    let deadline = tokio::time::Instant::now() + timeout;

    // Register the Notified future BEFORE reading the snapshot.
    // Notify::notify_waiters() only wakes already-registered waiters and does
    // not store a permit. Creating the future first ensures we don't miss a
    // notification that fires between the snapshot read and the select! poll.
    let mut notified = Box::pin(notify.notified());

    // Get current snapshot.
    let snapshot = engine.store().get(&id)?;

    // If already terminal, return immediately.
    if snapshot.status.is_terminal() {
        return Ok(Json(snapshot));
    }

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            let current = engine.store().get(&id)?;
            return Ok(Json(current));
        }

        tokio::select! {
            _ = &mut notified => {
                // Something changed — check if *this* task changed.
                let current = engine.store().get(&id)?;
                if current.status != snapshot.status
                    || current.progress != snapshot.progress
                    || current.message != snapshot.message
                {
                    return Ok(Json(current));
                }
                // Not our task — re-register and keep waiting.
                notified = Box::pin(notify.notified());
            }
            _ = tokio::time::sleep(remaining) => {
                let current = engine.store().get(&id)?;
                return Ok(Json(current));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// POST /tasks/:id/@cancel
// ---------------------------------------------------------------------------

async fn cancel_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.cancel(&id).await?;
    Ok(Json(task))
}

// ---------------------------------------------------------------------------
// DELETE /tasks/:id
// ---------------------------------------------------------------------------

async fn delete_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    engine.store().delete(&id)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
