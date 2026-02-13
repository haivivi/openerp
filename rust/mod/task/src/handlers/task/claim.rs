//! POST /task/tasks/:id/@claim — executor claims a task.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;
use crate::model::ClaimRequest;

pub async fn claim(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<ClaimRequest>,
) -> impl IntoResponse {
    match engine.claim(&id, &body.claimed_by) {
        Ok(task) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
