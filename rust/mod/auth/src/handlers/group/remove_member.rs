//! DELETE /auth/groups/:id/@members/:member_ref — remove a member.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::service::AuthService;

pub async fn remove_member(
    State(svc): State<Arc<AuthService>>,
    Path((id, member_ref)): Path<(String, String)>,
) -> impl IntoResponse {
    match svc.remove_group_member(&id, &member_ref) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => openerp_core::ServiceError::from(e).into_response(),
    }
}
