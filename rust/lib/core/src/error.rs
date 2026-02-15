use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

// ── Error codes ─────────────────────────────────────────────────────
//
// Stable, machine-readable identifiers. Clients match on these —
// never on the human-readable message string.

/// Stable error code constants.
///
/// Clients should match on `code` from `{"code": "NOT_FOUND", "message": "..."}`.
/// Codes never change; messages may be reworded.
pub mod error_code {
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const ALREADY_EXISTS: &str = "ALREADY_EXISTS";
    pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
    pub const UNAUTHENTICATED: &str = "UNAUTHENTICATED";
    pub const PERMISSION_DENIED: &str = "PERMISSION_DENIED";
    pub const READ_ONLY: &str = "READ_ONLY";
    pub const INTERNAL: &str = "INTERNAL";
    pub const STORAGE_ERROR: &str = "STORAGE_ERROR";
}

// ── ServiceError ────────────────────────────────────────────────────

/// Unified service error type used across all modules.
///
/// Each variant maps to a stable error code (see [`error_code`]) and an
/// HTTP status code. The JSON response always includes both:
///
/// ```json
/// {"code": "NOT_FOUND", "message": "employee 'abc' not found"}
/// ```
#[derive(Error, Debug)]
pub enum ServiceError {
    /// Resource does not exist. HTTP 404.
    #[error("{0}")]
    NotFound(String),

    /// Duplicate key / resource already exists. HTTP 409.
    #[error("{0}")]
    Conflict(String),

    /// Input data is invalid. HTTP 400.
    #[error("{0}")]
    Validation(String),

    /// Missing or invalid authentication credentials. HTTP 401.
    #[error("{0}")]
    Unauthorized(String),

    /// Authenticated but lacks required permission. HTTP 403.
    #[error("{0}")]
    PermissionDenied(String),

    /// Attempted write to read-only data. HTTP 403.
    #[error("{0}")]
    ReadOnly(String),

    /// Storage backend failure. HTTP 500.
    #[error("{0}")]
    Storage(String),

    /// Unexpected internal error. HTTP 500.
    #[error("{0}")]
    Internal(String),
}

impl ServiceError {
    /// Stable, machine-readable error code.
    pub fn error_code(&self) -> &'static str {
        match self {
            ServiceError::NotFound(_) => error_code::NOT_FOUND,
            ServiceError::Conflict(_) => error_code::ALREADY_EXISTS,
            ServiceError::Validation(_) => error_code::VALIDATION_FAILED,
            ServiceError::Unauthorized(_) => error_code::UNAUTHENTICATED,
            ServiceError::PermissionDenied(_) => error_code::PERMISSION_DENIED,
            ServiceError::ReadOnly(_) => error_code::READ_ONLY,
            ServiceError::Storage(_) => error_code::STORAGE_ERROR,
            ServiceError::Internal(_) => error_code::INTERNAL,
        }
    }

    /// HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
            ServiceError::Conflict(_) => StatusCode::CONFLICT,
            ServiceError::Validation(_) => StatusCode::BAD_REQUEST,
            ServiceError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ServiceError::PermissionDenied(_) => StatusCode::FORBIDDEN,
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
            "code": self.error_code(),
            "message": self.to_string(),
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
        assert_eq!(ServiceError::Unauthorized("x".into()).status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ServiceError::PermissionDenied("x".into()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ServiceError::ReadOnly("x".into()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ServiceError::Storage("x".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ServiceError::Internal("x".into()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn error_code_mapping() {
        assert_eq!(ServiceError::NotFound("x".into()).error_code(), "NOT_FOUND");
        assert_eq!(ServiceError::Conflict("x".into()).error_code(), "ALREADY_EXISTS");
        assert_eq!(ServiceError::Validation("x".into()).error_code(), "VALIDATION_FAILED");
        assert_eq!(ServiceError::Unauthorized("x".into()).error_code(), "UNAUTHENTICATED");
        assert_eq!(ServiceError::PermissionDenied("x".into()).error_code(), "PERMISSION_DENIED");
        assert_eq!(ServiceError::ReadOnly("x".into()).error_code(), "READ_ONLY");
        assert_eq!(ServiceError::Storage("x".into()).error_code(), "STORAGE_ERROR");
        assert_eq!(ServiceError::Internal("x".into()).error_code(), "INTERNAL");
    }

    #[test]
    fn json_response_format() {
        let err = ServiceError::NotFound("employee 'abc' not found".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn error_display_is_just_message() {
        // Display no longer has "not found:" prefix — just the message.
        assert_eq!(ServiceError::NotFound("user 123".into()).to_string(), "user 123");
        assert_eq!(ServiceError::Conflict("dup key".into()).to_string(), "dup key");
        assert_eq!(ServiceError::Validation("bad input".into()).to_string(), "bad input");
        assert_eq!(ServiceError::Unauthorized("missing token".into()).to_string(), "missing token");
        assert_eq!(ServiceError::PermissionDenied("no access".into()).to_string(), "no access");
    }
}
