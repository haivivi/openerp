use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Unified service error type used across all modules.
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("read-only: {0}")]
    ReadOnly(String),

    #[error("storage: {0}")]
    Storage(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl ServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
            ServiceError::Conflict(_) => StatusCode::CONFLICT,
            ServiceError::Validation(_) => StatusCode::BAD_REQUEST,
            ServiceError::ReadOnly(_) => StatusCode::FORBIDDEN,
            ServiceError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServiceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = serde_json::json!({
            "error": self.to_string(),
        });
        (status, axum::Json(body)).into_response()
    }
}
