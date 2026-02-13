//! POST /auth/sessions/:id/@revoke — revoke a session.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::service::AuthService;

/// Revoke a session by ID.
pub async fn revoke(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.revoke_session(&id) {
        Ok(session) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&session).unwrap())).into_response()
        }
        Err(e) => {
            openerp_core::ServiceError::from(e).into_response()
        }
    }
}
