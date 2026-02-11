use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};

use openerp_core::ServiceError;

use crate::engine::TaskEngine;
use crate::model::{RegisterTaskTypeRequest, TaskType};

type EngineState = Arc<TaskEngine>;

pub fn router(engine: Arc<TaskEngine>) -> Router {
    Router::new()
        .route("/task-types", get(list_types).post(register_type))
        .route("/task-types/{type_key}", delete(unregister_type))
        .with_state(engine)
}

// ---------------------------------------------------------------------------
// GET /task-types
// ---------------------------------------------------------------------------

async fn list_types(
    State(engine): State<EngineState>,
) -> Result<Json<Vec<TaskType>>, ServiceError> {
    let types = engine.task_types().await;
    Ok(Json(types))
}

// ---------------------------------------------------------------------------
// POST /task-types
// ---------------------------------------------------------------------------

async fn register_type(
    State(engine): State<EngineState>,
    Json(req): Json<RegisterTaskTypeRequest>,
) -> Result<Json<TaskType>, ServiceError> {
    let type_def = TaskType {
        task_type: req.task_type,
        service: req.service,
        description: req.description,
        default_timeout: req.default_timeout,
        max_concurrency: req.max_concurrency,
    };

    // Register with a no-op handler. Real handlers are registered
    // programmatically by business services. The HTTP endpoint is
    // mainly for runtime type metadata registration.
    let td = type_def.clone();
    engine
        .register(td, |_task, _ctx| async {
            Err(ServiceError::Internal(
                "no handler registered via API".into(),
            ))
        })
        .await;

    Ok(Json(type_def))
}

// ---------------------------------------------------------------------------
// DELETE /task-types/:type_key
// ---------------------------------------------------------------------------

async fn unregister_type(
    State(engine): State<EngineState>,
    Path(type_key): Path<String>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let removed = engine.unregister(&type_key).await;
    if !removed {
        return Err(ServiceError::NotFound(format!(
            "task type {type_key}"
        )));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}
