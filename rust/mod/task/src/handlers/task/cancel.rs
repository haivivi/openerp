//! POST /task/tasks/:id/@cancel — cancel a pending task.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;

pub async fn cancel(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match engine.cancel(&id) {
        Ok(task) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
