//! GET /auth/groups/:id/@members — list group members.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::service::AuthService;

pub async fn list_members(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match svc.list_group_members(&id) {
        Ok(members) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&members).unwrap())).into_response()
        }
        Err(e) => openerp_core::ServiceError::from(e).into_response(),
    }
}
