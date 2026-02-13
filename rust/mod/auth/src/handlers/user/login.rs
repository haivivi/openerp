//! POST /auth/oauth/callback — OAuth login callback.
//!
//! Processes OAuth provider callback with authorization code.
//! Exchanges code for token, fetches userinfo, creates/finds user, issues JWT.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::service::AuthService;

/// OAuth login callback handler.
///
/// Flow: code → oauth_callback → find_or_create_oauth_user → issue_tokens.
pub async fn login(
    State(svc): State<Arc<AuthService>>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> impl IntoResponse {
    let provider_id = body["provider"].as_str().unwrap_or("");
    let code = body["code"].as_str().unwrap_or("");

    if provider_id.is_empty() || code.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "missing 'provider' and 'code' fields"
            })),
        ).into_response();
    }

    // Step 1: Exchange code for user info via OAuth provider
    let user_info = match svc.oauth_callback(provider_id, code).await {
        Ok(info) => info,
        Err(e) => {
            let se: openerp_core::ServiceError = e.into();
            return se.into_response();
        }
    };

    // Step 2: Find or create user from OAuth info
    let user = match svc.find_or_create_oauth_user(provider_id, &user_info) {
        Ok(u) => u,
        Err(e) => {
            let se: openerp_core::ServiceError = e.into();
            return se.into_response();
        }
    };

    // Step 3: Issue JWT tokens
    match svc.issue_tokens(&user) {
        Ok(token_pair) => {
            (StatusCode::OK, axum::Json(serde_json::to_value(&token_pair).unwrap())).into_response()
        }
        Err(e) => {
            let se: openerp_core::ServiceError = e.into();
            se.into_response()
        }
    }
}
