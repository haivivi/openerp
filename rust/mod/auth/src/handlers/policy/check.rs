//! POST /auth/check — check permission for a subject.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::model::CheckParams;
use crate::service::AuthService;

pub async fn check(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<CheckParams>,
) -> impl IntoResponse {
    match svc.check_permission(&body) {
        Ok(result) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&result).unwrap())).into_response()
        }
        Err(e) => openerp_core::ServiceError::from(e).into_response(),
    }
}
