//! POST /task/tasks/:id/@log — write log entries.
//! GET /task/tasks/:id/@logs — read log entries.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::engine::TaskEngine;
use crate::model::{LogQuery, LogRequest};

pub async fn log_write(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<LogRequest>,
) -> impl IntoResponse {
    match engine.append_log(&id, &body.level, &body.lines) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn log_read(
    State(engine): State<Arc<TaskEngine>>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> impl IntoResponse {
    match engine.query_logs(&id, query.limit, query.desc, query.level.as_deref()) {
        Ok(entries) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&entries).unwrap())).into_response()
        }
        Err(e) => e.into_response(),
    }
}
