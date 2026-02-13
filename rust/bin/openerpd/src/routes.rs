//! Route registration — collects all module routes + system endpoints.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use axum::middleware;

use crate::auth_middleware::{self, JwtState};
use crate::login;

/// Application shared state.
#[derive(Clone)]
pub struct AppState {
    pub jwt_state: Arc<JwtState>,
    pub server_config: Arc<crate::config::ServerConfig>,
    pub kv: Arc<dyn openerp_kv::KVStore>,
    pub sql: Arc<dyn openerp_sql::SQLStore>,
}

/// Build the complete router with all routes.
pub fn build_router(state: AppState) -> Router {
    let jwt_state = state.jwt_state.clone();

    // System endpoints (public).
    let system_routes = Router::new()
        .route("/", get(index_page))
        .route("/health", get(health))
        .route("/version", get(version));

    // Auth login endpoints (public).
    let auth_login_routes = login::routes(state.clone());

    // Module routes — will be added as modules are wired up.
    // Each module's routes are nested under /{module_name}.
    let module_routes = Router::new();

    // Combine everything.
    Router::new()
        .merge(system_routes)
        .merge(auth_login_routes)
        .merge(module_routes)
        .layer(middleware::from_fn_with_state(
            jwt_state,
            auth_middleware::auth_middleware,
        ))
        .with_state(state)
}

async fn index_page() -> impl IntoResponse {
    Html(include_str!("web/login.html"))
}

async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
    }))
}

async fn version() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "name": "openerpd",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
