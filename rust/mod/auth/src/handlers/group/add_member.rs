//! POST /auth/groups/:id/@members — add a member to a group.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::model::AddGroupMember;
use crate::service::AuthService;

pub async fn add_member(
    State(svc): State<Arc<AuthService>>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<AddGroupMember>,
) -> impl IntoResponse {
    match svc.add_group_member(&id, body) {
        Ok(member) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&member).unwrap())).into_response()
        }
        Err(e) => openerp_core::ServiceError::from(e).into_response(),
    }
}
