//! Route registration — collects all module routes + system endpoints.

use std::sync::Arc;

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
pub fn build_router(state: AppState, module_routes: Vec<(&str, Router)>) -> Router {
    let jwt_state = state.jwt_state.clone();

    // System endpoints (public, no state needed).
    let system_routes = Router::new()
        .route("/health", get(health))
        .route("/version", get(version));

    // Start with the system and login routes (which need AppState).
    let mut app: Router<()> = Router::new()
        .route("/", get(index_page))
        .merge(login::routes(state.clone()))
        .with_state(state);

    // Merge stateless system routes.
    app = app.merge(system_routes);

    // Mount each module's routes under /{module_name}.
    // Module routes are already Router<()> (they called .with_state() internally).
    for (name, router) in module_routes {
        app = app.nest(&format!("/{}", name), router);
    }

    // Apply JWT auth middleware to all routes.
    app.layer(middleware::from_fn_with_state(
        jwt_state,
        auth_middleware::auth_middleware,
    ))
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
