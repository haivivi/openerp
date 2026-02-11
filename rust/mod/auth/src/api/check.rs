use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};

use openerp_core::ServiceError;

use crate::model::CheckParams;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/check", get(check_permission))
}

/// GET /auth/check?who=user:alice&what=pms:batch:B001&how=write
///
/// Checks if the subject has the requested permission.
/// Returns { "allowed": true/false, "policy_id": "..." }.
async fn check_permission(
    State(svc): State<AppState>,
    Query(params): Query<CheckParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let result = svc
        .check_permission(&params)
        .map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}
