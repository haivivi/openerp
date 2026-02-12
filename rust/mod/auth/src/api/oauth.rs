use axum::extract::{Extension, Path, Query, State};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum::{Json, Router};

use openerp_core::ServiceError;

use crate::model::{Claims, RefreshRequest};
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login/{provider}", get(login))
        .route("/callback/{provider}", get(callback))
        .route("/token/refresh", post(refresh))
        .route("/token/revoke", post(revoke))
}

/// Redirect to OAuth provider's authorization URL.
/// GET /auth/login/{provider}
async fn login(
    State(svc): State<AppState>,
    Path(provider): Path<String>,
) -> Result<Redirect, ServiceError> {
    let state = openerp_core::new_id();
    let url = svc
        .oauth_authorize_url(&provider, &state)
        .map_err(ServiceError::from)?;
    Ok(Redirect::temporary(&url))
}

/// OAuth callback — exchange code for tokens.
/// GET /auth/callback/{provider}?code=...&state=...
async fn callback(
    State(svc): State<AppState>,
    Path(provider): Path<String>,
    Query(params): Query<CallbackParams>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let user_info = svc
        .oauth_callback(&provider, &params.code)
        .await
        .map_err(ServiceError::from)?;

    let user = svc
        .find_or_create_oauth_user(&provider, &user_info)
        .map_err(ServiceError::from)?;

    let tokens = svc.issue_tokens(&user).map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(tokens).unwrap()))
}

/// Refresh access token.
/// POST /auth/token/refresh
async fn refresh(
    State(svc): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let tokens = svc
        .refresh_tokens(&body.refresh_token)
        .map_err(ServiceError::from)?;
    Ok(Json(serde_json::to_value(tokens).unwrap()))
}

/// Revoke the current session.
/// POST /auth/token/revoke
async fn revoke(
    State(svc): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<axum::http::StatusCode, ServiceError> {
    svc.revoke_session(&claims.sid).map_err(ServiceError::from)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct CallbackParams {
    code: String,
    #[allow(dead_code)]
    #[serde(default)]
    state: String,
}
