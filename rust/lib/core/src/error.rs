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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_mapping() {
        assert_eq!(ServiceError::NotFound("x".into()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ServiceError::Conflict("x".into()).status_code(), StatusCode::CONFLICT);
        assert_eq!(ServiceError::Validation("x".into()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ServiceError::ReadOnly("x".into()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ServiceError::Storage("x".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ServiceError::Internal("x".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn error_display_messages() {
        assert_eq!(ServiceError::NotFound("user 123".into()).to_string(), "not found: user 123");
        assert_eq!(ServiceError::Conflict("dup key".into()).to_string(), "conflict: dup key");
        assert_eq!(ServiceError::Validation("bad input".into()).to_string(), "validation: bad input");
        assert_eq!(ServiceError::ReadOnly("key1".into()).to_string(), "read-only: key1");
        assert_eq!(ServiceError::Storage("disk full".into()).to_string(), "storage: disk full");
        assert_eq!(ServiceError::Internal("panic".into()).to_string(), "internal: panic");
    }
}
