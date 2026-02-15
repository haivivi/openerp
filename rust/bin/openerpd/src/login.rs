//! Login endpoint — root password or user email+password.

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

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

pub fn routes(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login_handler))
}

async fn login_handler(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<LoginRequest>,
) -> impl IntoResponse {
    // Root login.
    if body.username == "root" {
        return handle_root_login(&state, &body.password);
    }

    // User login: username is email.
    handle_user_login(&state, &body.username, &body.password)
}

fn handle_root_login(state: &AppState, password: &str) -> axum::response::Response {
    let config = &state.server_config;

    if !verify_root_password(password, &config.root.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "invalid credentials"})),
        ).into_response();
    }

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

    issue_jwt(state, &claims, expire_secs)
}

fn handle_user_login(state: &AppState, email: &str, password: &str) -> axum::response::Response {
    // Find user by email.
    let user = match auth::store_impls::find_user_by_email(&state.kv, email) {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "invalid credentials"})),
            ).into_response();
        }
        Err(e) => {
            tracing::error!("Failed to find user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": "internal server error"})),
            ).into_response();
        }
    };

    // Check password.
    let hash = match &user.password_hash {
        Some(h) if !h.is_empty() => h.as_str(),
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "account has no password set"})),
            ).into_response();
        }
    };

    if !auth::store_impls::verify_password(password, hash) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({"error": "invalid credentials"})),
        ).into_response();
    }

    if !user.active {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error": "account is deactivated"})),
        ).into_response();
    }

    // Build claims.
    let config = &state.server_config;
    let now = chrono::Utc::now().timestamp();
    let expire_secs = config.jwt.expire_secs;

    // Look up roles for this user from policies.
    let roles = auth::store_impls::find_roles_for_user(&state.kv, user.id.as_str())
        .unwrap_or_default();

    let display = user.display_name.as_deref().unwrap_or("User");
    let claims = Claims {
        sub: user.id.to_string(),
        name: display.to_string(),
        groups: vec![],
        roles,
        sid: openerp_core::new_id(),
        iat: now,
        exp: now + expire_secs as i64,
    };

    issue_jwt(state, &claims, expire_secs)
}

fn issue_jwt(state: &AppState, claims: &Claims, expire_secs: u64) -> axum::response::Response {
    let encoding_key = EncodingKey::from_secret(state.server_config.jwt.secret.as_bytes());
    match encode(&Header::default(), claims, &encoding_key) {
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
                axum::Json(serde_json::json!({"error": "internal server error"})),
            ).into_response()
        }
    }
}
