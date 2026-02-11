use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, middleware::Next};
use serde_json::json;

use crate::api::AppState;

/// Paths that don't require authentication.
const PUBLIC_PATHS: &[&str] = &[
    "/auth/login/",
    "/auth/callback/",
    "/auth/token/refresh",
    "/auth/providers",
];

/// JWT authentication middleware.
///
/// Checks for a Bearer token in the Authorization header.
/// Public paths (login, callback, providers list) are excluded.
/// If valid, stores Claims as Extension for handlers to access via `Extension<Claims>`.
pub async fn auth_middleware(
    State(svc): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    // Check if path is public
    if is_public_path(&path) {
        return next.run(req).await;
    }

    // Extract Bearer token
    let token = match extract_bearer(req.headers()) {
        Some(t) => t.to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "missing authorization header"})),
            )
                .into_response();
        }
    };

    // Verify token
    match svc.verify_token(&token) {
        Ok(claims) => {
            // Store claims as extension for handlers to extract
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Extract the Bearer token from Authorization header.
fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Check if a path is public (no auth required).
fn is_public_path(path: &str) -> bool {
    for prefix in PUBLIC_PATHS {
        if path.starts_with(prefix) {
            return true;
        }
    }
    false
}
