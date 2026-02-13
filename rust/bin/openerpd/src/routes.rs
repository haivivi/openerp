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
        .route("/version", get(version))
        .route("/meta/schema", get(schema_endpoint));

    // Start with the system and login routes (which need AppState).
    let mut app: Router<()> = Router::new()
        .route("/", get(index_page))
        .route("/dashboard", get(dashboard_page))
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

async fn dashboard_page() -> impl IntoResponse {
    Html(include_str!("web/dashboard.html"))
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

/// Serve the DSL schema as JSON for the frontend.
/// Collects embedded IR from all DSL-defined modules.
async fn schema_endpoint() -> impl IntoResponse {
    use auth_dsl::model as auth_models;

    // Build schema from embedded IR consts.
    let auth_module = serde_json::json!({
        "id": "auth",
        "label": "Authentication",
        "icon": "shield",
        "resources": [
            serde_json::from_str::<serde_json::Value>(auth_models::User::__DSL_IR).unwrap_or_default(),
            serde_json::from_str::<serde_json::Value>(auth_models::Role::__DSL_IR).unwrap_or_default(),
            serde_json::from_str::<serde_json::Value>(auth_models::Group::__DSL_IR).unwrap_or_default(),
            serde_json::from_str::<serde_json::Value>(auth_models::Policy::__DSL_IR).unwrap_or_default(),
            serde_json::from_str::<serde_json::Value>(auth_models::Session::__DSL_IR).unwrap_or_default(),
            serde_json::from_str::<serde_json::Value>(auth_models::Provider::__DSL_IR).unwrap_or_default(),
        ],
        "hierarchy": {
            "nav": [
                {"model": "User", "path": "/users", "label": "Users", "icon": "users"},
                {"model": "Role", "path": "/roles", "label": "Roles", "icon": "shield"},
                {"model": "Group", "path": "/groups", "label": "Groups", "icon": "layers"},
                {"model": "Policy", "path": "/policies", "label": "Policies", "icon": "lock"},
                {"model": "Session", "path": "/sessions", "label": "Sessions", "icon": "clock"},
                {"model": "Provider", "path": "/providers", "label": "Providers", "icon": "globe"},
            ]
        },
        "facets": ["data"]
    });

    axum::Json(serde_json::json!({
        "name": "OpenERP",
        "modules": [auth_module],
    }))
}
