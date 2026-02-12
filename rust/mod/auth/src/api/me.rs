use axum::extract::{Extension, State};
use axum::routing::get;
use axum::{Json, Router};

use openerp_core::ServiceError;

use crate::model::Claims;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/me/groups", get(my_groups))
        .route("/me/roles", get(my_roles))
        .route("/me/permissions", get(my_permissions))
}

/// GET /auth/me — current user info from JWT claims.
async fn me(
    State(svc): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let user = svc.get_user(&claims.sub).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

/// GET /auth/me/groups — groups the current user belongs to.
async fn my_groups(
    State(svc): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let groups = svc
        .get_user_direct_groups(&claims.sub)
        .map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": groups})))
}

/// GET /auth/me/roles — roles assigned to the current user.
async fn my_roles(
    State(svc): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let roles = svc
        .get_user_roles(&claims.sub)
        .map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": roles})))
}

/// GET /auth/me/permissions — all expanded permissions for the current user.
async fn my_permissions(
    State(svc): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let permissions = svc
        .get_user_permissions(&claims.sub)
        .map_err(ServiceError::from)?;
    Ok(Json(serde_json::json!({"items": permissions})))
}
