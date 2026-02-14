//! Route registration — admin routes + system endpoints.

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
}

/// Build the complete router.
pub fn build_router(
    state: AppState,
    admin_routes: Vec<(&str, Router)>,
    schema_json: serde_json::Value,
) -> Router {
    let jwt_state = state.jwt_state.clone();

    // System endpoints (public).
    let system_routes = Router::new()
        .route("/health", get(health))
        .route("/version", get(version));

    // Schema endpoint.
    let schema = schema_json.clone();
    let schema_route = Router::new().route(
        "/meta/schema",
        get(move || {
            let s = schema.clone();
            async move { axum::Json(s) }
        }),
    );

    // App shell + login.
    let mut app: Router<()> = Router::new()
        .route("/", get(index_page))
        .route("/dashboard", get(dashboard_page))
        .merge(login::routes(state.clone()))
        .with_state(state);

    app = app.merge(system_routes);
    app = app.merge(schema_route);

    // Mount admin routes from DSL modules.
    for (name, router) in admin_routes {
        app = app.nest(&format!("/admin/{}", name), router);
    }

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
    axum::Json(serde_json::json!({"status": "ok"}))
}

async fn version() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "name": "openerpd",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
