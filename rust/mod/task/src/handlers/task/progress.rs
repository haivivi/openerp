//! POST /task/tasks/:id/@progress — executor reports progress.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;
use crate::model::ProgressReport;

pub async fn progress(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<ProgressReport>,
) -> impl IntoResponse {
    match engine.report_progress(&id, body.total, body.success, body.failed, body.message) {
        Ok(task) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
