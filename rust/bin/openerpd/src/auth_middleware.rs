//! JWT authentication middleware + permission checking.
//!
//! Extracts JWT from `Authorization: Bearer <token>`, validates it,
//! and provides `Claims` to downstream handlers.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::bootstrap::ROOT_ROLE_ID;

/// JWT claims payload — mirrors auth::model::Claims but lives here
/// because openerpd is the binary that validates tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user id (or "root" for superadmin).
    pub sub: String,
    /// Display name.
    pub name: String,
    /// Groups the user belongs to.
    #[serde(default)]
    pub groups: Vec<String>,
    /// Roles assigned (via policies). Root has ["auth:root"].
    #[serde(default)]
    pub roles: Vec<String>,
    /// Session id.
    pub sid: String,
    /// Issued at (unix timestamp).
    pub iat: i64,
    /// Expiration (unix timestamp).
    pub exp: i64,
}

impl Claims {
    /// Check if this user is the virtual root superadmin.
    pub fn is_root(&self) -> bool {
        self.roles.iter().any(|r| r == ROOT_ROLE_ID)
    }
}

/// Shared JWT configuration for the middleware.
#[derive(Clone)]
pub struct JwtState {
    pub decoding_key: DecodingKey,
    pub validation: Validation,
}

/// Error type for authentication / authorization failures.
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    PermissionDenied(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "missing authorization token".to_string()),
            AuthError::InvalidToken(e) => (StatusCode::UNAUTHORIZED, format!("invalid token: {}", e)),
            AuthError::PermissionDenied(e) => (StatusCode::FORBIDDEN, format!("permission denied: {}", e)),
        };
        let body = serde_json::json!({ "error": msg });
        (status, axum::Json(body)).into_response()
    }
}

/// Middleware that extracts and validates JWT from the Authorization header.
///
/// If the request path is in the public list, the middleware passes through.
/// Otherwise, it requires a valid JWT and stores Claims in request extensions.
pub async fn auth_middleware(
    State(jwt_state): State<Arc<JwtState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let path = request.uri().path().to_string();

    // Public endpoints that don't require authentication.
    if is_public_path(&path) {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token.
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AuthError::MissingToken)?;

    // Validate and decode JWT.
    let token_data = jsonwebtoken::decode::<Claims>(
        token,
        &jwt_state.decoding_key,
        &jwt_state.validation,
    )
    .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

    // Store claims in request extensions for handlers to access.
    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}

/// Check if a request path is public (no auth required).
fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/" | "/dashboard" | "/health" | "/version" | "/meta/schema"
    ) || path.starts_with("/admin/") // Admin routes have their own Authenticator
      || path.starts_with("/mfg/")  // Facet routes handle their own auth
      || path.starts_with("/gear/") // Facet routes handle their own auth
      || path.starts_with("/auth/login")
      || path.starts_with("/auth/providers")
      || path.starts_with("/auth/oauth/")
      || path.starts_with("/auth/token/refresh")
      || path.starts_with("/assets/")
}

/// Check a specific permission for the given claims.
///
/// Root users (auth:root role) bypass all permission checks.
/// Normal users must have a matching policy.
pub fn check_permission(claims: &Claims, permission: &str) -> Result<(), AuthError> {
    // Root bypasses all checks.
    if claims.is_root() {
        return Ok(());
    }

    // TODO: integrate with PolicyService for real permission checking.
    // For now, we only check root. Full policy checking will be added
    // when the auth module's PolicyService is wired up to the middleware.
    Err(AuthError::PermissionDenied(format!(
        "user {} lacks permission {}",
        claims.sub, permission
    )))
}
