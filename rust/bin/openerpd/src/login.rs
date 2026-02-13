//! Root login endpoint — verifies password against argon2id hash, issues JWT.

use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::auth_middleware::Claims;
use crate::bootstrap::{verify_root_password, ROOT_ROLE_ID};
use crate::routes::AppState;

/// Login request body.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response body.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Register login routes.
pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login_handler))
}

/// Handle POST /auth/login.
///
/// For root: verify password against config hash, issue JWT with auth:root role.
/// For normal users: TODO — will delegate to auth module's UserService.
async fn login_handler(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<LoginRequest>,
) -> impl IntoResponse {
    if body.username == "root" {
        return handle_root_login(&state, &body.password).await;
    }

    // Normal user login — TODO: delegate to auth module.
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error": "User login not yet implemented. Use root account."
        })),
    ).into_response()
}

async fn handle_root_login(state: &AppState, password: &str) -> axum::response::Response {
    let config = &state.server_config;

    if !verify_root_password(password, &config.root.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "invalid credentials"
            })),
        ).into_response();
    }

    // Issue JWT with root claims.
    let now = chrono::Utc::now().timestamp();
    let expire_secs = config.jwt.expire_secs;

    let claims = Claims {
        sub: "root".to_string(),
        name: "Root".to_string(),
        groups: vec![],
        roles: vec![ROOT_ROLE_ID.to_string()],
        sid: openerp_core::new_id(),
        iat: now,
        exp: now + expire_secs as i64,
    };

    let encoding_key = EncodingKey::from_secret(config.jwt.secret.as_bytes());
    match encode(&Header::default(), &claims, &encoding_key) {
        Ok(token) => {
            let response = LoginResponse {
                access_token: token,
                token_type: "Bearer".to_string(),
                expires_in: expire_secs,
            };
            (StatusCode::OK, axum::Json(serde_json::json!(response))).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to encode JWT: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "error": "internal server error"
                })),
            ).into_response()
        }
    }
}
