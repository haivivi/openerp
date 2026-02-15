//! Task lifecycle action handlers.
//!
//! Actions: claim, progress, complete, fail, cancel.
//! All operate on a Task by ID and transition its status.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use openerp_core::ServiceError;
use openerp_store::KvOps;
use openerp_types::DateTime;

use crate::model::Task;

pub fn routes(kv: Arc<dyn openerp_kv::KVStore>) -> Router {
    let ops = Arc::new(KvOps::<Task>::new(kv));
    Router::new()
        .route("/tasks/{id}/@claim", post(claim))
        .route("/tasks/{id}/@progress", post(progress))
        .route("/tasks/{id}/@complete", post(complete))
        .route("/tasks/{id}/@fail", post(fail))
        .route("/tasks/{id}/@cancel", post(cancel))
        .with_state(ops)
}

// ── Request/Response types ──

#[derive(serde::Deserialize)]
pub struct ClaimRequest {
    pub worker_id: String,
}

#[derive(serde::Deserialize)]
pub struct ProgressRequest {
    pub success: Option<i64>,
    pub failed: Option<i64>,
    pub message: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct FailRequest {
    pub error: String,
}

#[derive(serde::Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub status: String,
}

fn now() -> DateTime {
    DateTime::new(&chrono::Utc::now().to_rfc3339())
}

fn task_response(task: &Task) -> TaskResponse {
    TaskResponse {
        id: task.id.to_string(),
        status: task.status.clone(),
    }
}

// ── Handlers ──

/// Claim a pending task for processing.
async fn claim(
    State(ops): State<Arc<KvOps<Task>>>,
    Path(id): Path<String>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<TaskResponse>, ServiceError> {
    let mut task = ops.get_or_err(&id)?;

    if task.status != "pending" {
        return Err(ServiceError::Validation(format!(
            "cannot claim task in '{}' status, must be 'pending'",
            task.status
        )));
    }

    task.status = "running".into();
    task.claimed_by = Some(req.worker_id);
    task.started_at = Some(now());
    task.last_active_at = Some(now());
    ops.save(task.clone())?;

    Ok(Json(task_response(&task)))
}

/// Report progress on a running task.
async fn progress(
    State(ops): State<Arc<KvOps<Task>>>,
    Path(id): Path<String>,
    Json(req): Json<ProgressRequest>,
) -> Result<Json<TaskResponse>, ServiceError> {
    let mut task = ops.get_or_err(&id)?;

    if task.status != "running" {
        return Err(ServiceError::Validation(format!(
            "cannot report progress on '{}' task",
            task.status
        )));
    }

    if let Some(s) = req.success {
        task.success += s;
    }
    if let Some(f) = req.failed {
        task.failed += f;
    }
    if let Some(m) = req.message {
        task.message = Some(m);
    }
    task.last_active_at = Some(now());
    ops.save(task.clone())?;

    Ok(Json(task_response(&task)))
}

/// Mark a task as successfully completed.
async fn complete(
    State(ops): State<Arc<KvOps<Task>>>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, ServiceError> {
    let mut task = ops.get_or_err(&id)?;

    if task.status != "running" {
        return Err(ServiceError::Validation(format!(
            "cannot complete task in '{}' status",
            task.status
        )));
    }

    task.status = "completed".into();
    task.ended_at = Some(now());
    task.last_active_at = Some(now());
    ops.save(task.clone())?;

    Ok(Json(task_response(&task)))
}

/// Mark a task as failed.
async fn fail(
    State(ops): State<Arc<KvOps<Task>>>,
    Path(id): Path<String>,
    Json(req): Json<FailRequest>,
) -> Result<Json<TaskResponse>, ServiceError> {
    let mut task = ops.get_or_err(&id)?;

    if task.status != "running" {
        return Err(ServiceError::Validation(format!(
            "cannot fail task in '{}' status",
            task.status
        )));
    }

    task.status = "failed".into();
    task.error = Some(req.error);
    task.ended_at = Some(now());
    task.last_active_at = Some(now());

    // Check retry.
    if task.retry_count < task.max_retries {
        task.retry_count += 1;
        task.status = "pending".into();
        task.ended_at = None;
        task.claimed_by = None;
    }

    ops.save(task.clone())?;

    Ok(Json(task_response(&task)))
}

/// Cancel a pending or running task.
async fn cancel(
    State(ops): State<Arc<KvOps<Task>>>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, ServiceError> {
    let mut task = ops.get_or_err(&id)?;

    match task.status.as_str() {
        "pending" | "running" => {}
        other => {
            return Err(ServiceError::Validation(format!(
                "cannot cancel task in '{}' status",
                other
            )));
        }
    }

    task.status = "cancelled".into();
    task.ended_at = Some(now());
    task.last_active_at = Some(now());
    ops.save(task.clone())?;

    Ok(Json(task_response(&task)))
}
