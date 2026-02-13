//! POST /task/tasks/:id/@complete — executor marks task as completed.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;
use crate::model::CompleteRequest;

pub async fn complete(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<CompleteRequest>,
) -> impl IntoResponse {
    match engine.complete(&id, body.message.as_deref()) {
        Ok(task) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
