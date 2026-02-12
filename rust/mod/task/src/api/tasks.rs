use std::sync::Arc;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};

use openerp_core::ServiceError;

use crate::engine::TaskEngine;
use crate::model::{
    ClaimRequest, CompleteRequest, CreateTaskRequest, FailRequest, LogQuery, LogRequest,
    PollQuery, Task, TaskListQuery,
};

type EngineState = Arc<TaskEngine>;

pub fn router(engine: Arc<TaskEngine>) -> Router {
    Router::new()
        // Caller-facing
        .route("/tasks", post(create_task).get(list_tasks))
        .route("/tasks/{id}", get(get_task).delete(delete_task))
        .route("/tasks/{id}/@poll", get(poll_task))
        .route("/tasks/{id}/@cancel", post(cancel_task))
        .route("/tasks/{id}/@logs", get(get_logs))
        // Executor-facing
        .route("/tasks/{id}/@claim", post(claim_task))
        .route("/tasks/{id}/@progress", post(report_progress))
        .route("/tasks/{id}/@heartbeat", post(heartbeat))
        .route("/tasks/{id}/@complete", post(complete_task))
        .route("/tasks/{id}/@fail", post(fail_task))
        .route("/tasks/{id}/@data", get(load_data).put(save_data))
        .route("/tasks/{id}/@input", get(load_input))
        .route("/tasks/{id}/@log", post(pipe_log))
        .with_state(engine)
}

// ===========================================================================
// Caller-facing endpoints
// ===========================================================================

/// POST /tasks — create a new task.
async fn create_task(
    State(engine): State<EngineState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<Task>), ServiceError> {
    let task = engine
        .create_task(
            &req.task_type,
            &req.input,
            req.timeout_secs,
            req.max_retries,
            req.created_by,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(task)))
}

/// GET /tasks — list tasks with optional filters.
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

/// GET /tasks/:id — get a single task.
async fn get_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.store().get(&id)?;
    Ok(Json(task))
}

/// GET /tasks/:id/@poll — long-poll for state change.
async fn poll_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Query(query): Query<PollQuery>,
) -> Result<Json<Task>, ServiceError> {
    let timeout = Duration::from_secs(query.timeout.min(120));
    let notify = engine.notify().clone();
    let deadline = tokio::time::Instant::now() + timeout;

    // Register the Notified future BEFORE reading the snapshot to avoid
    // missing a notification that fires between snapshot and select!.
    let mut notified = Box::pin(notify.notified());

    let snapshot = engine.store().get(&id)?;

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
                let current = engine.store().get(&id)?;
                if current.status != snapshot.status
                    || current.total != snapshot.total
                    || current.success != snapshot.success
                    || current.failed != snapshot.failed
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

/// POST /tasks/:id/@cancel — cancel a task.
async fn cancel_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.cancel(&id)?;
    Ok(Json(task))
}

/// DELETE /tasks/:id — delete a task.
async fn delete_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    engine.store().delete(&id)?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// GET /tasks/:id/@logs — read task logs.
async fn get_logs(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let limit = query.limit.unwrap_or(100);
    let entries = engine
        .store()
        .query_logs(&id, query.level.as_deref(), limit, query.desc)?;
    Ok(Json(serde_json::json!({ "items": entries })))
}

// ===========================================================================
// Executor-facing endpoints
// ===========================================================================

/// POST /tasks/:id/@claim — claim a task (PENDING -> RUNNING).
async fn claim_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.claim(&id, &req.claimed_by)?;
    Ok(Json(task))
}

/// POST /tasks/:id/@progress — report progress counters.
async fn report_progress(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Json(req): Json<crate::model::ProgressReport>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.report_progress(&id, req.total, req.success, req.failed, req.message)?;
    Ok(Json(task))
}

/// POST /tasks/:id/@heartbeat — keep alive.
async fn heartbeat(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    engine.heartbeat(&id)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /tasks/:id/@complete — mark task as COMPLETED.
async fn complete_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Json(req): Json<CompleteRequest>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.complete(&id, req.message.as_deref())?;
    Ok(Json(task))
}

/// POST /tasks/:id/@fail — mark task as FAILED.
async fn fail_task(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Json(req): Json<FailRequest>,
) -> Result<Json<Task>, ServiceError> {
    let task = engine.fail(&id, &req.error, req.message.as_deref())?;
    Ok(Json(task))
}

/// GET /tasks/:id/@data — load executor runtime data.
async fn load_data(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ServiceError> {
    match engine.load_data(&id)? {
        Some(data) => Ok((
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            data,
        )),
        None => Ok((
            StatusCode::NO_CONTENT,
            [("content-type", "application/octet-stream")],
            vec![],
        )),
    }
}

/// PUT /tasks/:id/@data — save executor runtime data.
async fn save_data(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, ServiceError> {
    engine.save_data(&id, &body)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /tasks/:id/@input — load task input params.
async fn load_input(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ServiceError> {
    match engine.load_input(&id)? {
        Some(data) => Ok((
            StatusCode::OK,
            [("content-type", "application/json")],
            data,
        )),
        None => Ok((
            StatusCode::NO_CONTENT,
            [("content-type", "application/json")],
            vec![],
        )),
    }
}

/// POST /tasks/:id/@log — pipe log lines.
async fn pipe_log(
    State(engine): State<EngineState>,
    Path(id): Path<String>,
    Json(req): Json<LogRequest>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    engine.append_log(&id, &req.level, &req.lines)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
