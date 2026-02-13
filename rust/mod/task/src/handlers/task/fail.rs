//! POST /task/tasks/:id/@fail — executor marks task as failed.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;
use crate::model::FailRequest;

pub async fn fail(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<FailRequest>,
) -> impl IntoResponse {
    match engine.fail(&id, &body.error, body.message.as_deref()) {
        Ok(task) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
