//! GET /task/tasks/:id/@poll — long-poll for task state changes.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;
use crate::model::PollQuery;

pub async fn poll(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    Query(query): Query<PollQuery>,
) -> impl IntoResponse {
    match engine.poll(&id, query.timeout).await {
        Ok(task) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&task).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
