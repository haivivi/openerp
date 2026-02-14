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
pub fn build_router(
    state: AppState,
    module_routes: Vec<(&str, Router)>,
    admin_routes: Vec<(&str, Router)>,
    schema_json: serde_json::Value,
) -> Router {
    let jwt_state = state.jwt_state.clone();

    // System endpoints (public, no state needed).
    let system_routes = Router::new()
        .route("/health", get(health))
        .route("/version", get(version));

    // Schema endpoint — serves the DSL-generated schema JSON.
    let schema = schema_json.clone();
    let schema_route = Router::new().route(
        "/meta/schema",
        get(move || {
            let s = schema.clone();
            async move { axum::Json(s) }
        }),
    );

    // Start with the system and login routes (which need AppState).
    let mut app: Router<()> = Router::new()
        .route("/", get(index_page))
        .route("/dashboard", get(dashboard_page))
        .merge(login::routes(state.clone()))
        .with_state(state);

    // Merge stateless system routes.
    app = app.merge(system_routes);
    app = app.merge(schema_route);

    // Mount old module routes (auth, pms, task — will be replaced over time).
    for (name, router) in module_routes {
        app = app.nest(&format!("/{}", name), router);
    }

    // Mount admin routes from DSL modules.
    for (name, router) in admin_routes {
        app = app.nest(&format!("/admin/{}", name), router);
    }

    // Apply JWT auth middleware to all routes.
    app.layer(middleware::from_fn_with_state(
        jwt_state,
        auth_middleware::auth_middleware,
    ))
}

async fn index_page() -> impl IntoResponse {
    Html(openerp_web::login_html())
}

async fn dashboard_page() -> impl IntoResponse {
    Html(openerp_web::dashboard_html())
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
