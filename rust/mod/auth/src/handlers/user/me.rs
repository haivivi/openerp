//! GET /auth/me — return the current user's profile.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;

use crate::model::Claims;
use crate::service::AuthService;

/// Return the current authenticated user's profile.
pub async fn me(
    Extension(claims): Extension<Claims>,
    State(svc): State<Arc<AuthService>>,
) -> impl IntoResponse {
    // Root virtual user.
    if claims.sub == "root" {
        return (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "id": "root",
                "name": "Root",
                "email": null,
                "active": true,
                "roles": claims.roles,
                "groups": claims.groups,
            })),
        ).into_response();
    }

    match svc.get_user(&claims.sub) {
        Ok(user) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&user).unwrap())).into_response()
        }
        Err(e) => {
            openerp_core::ServiceError::from(e).into_response()
        }
    }
}
