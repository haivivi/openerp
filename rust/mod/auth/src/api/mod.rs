mod users;
mod groups;
mod providers;
mod roles;
mod policies;
mod oauth;
mod me;
mod check;
mod middleware;

use std::sync::Arc;

use axum::Router;

use crate::service::AuthService;

/// Shared application state.
pub type AppState = Arc<AuthService>;

/// Build the complete auth API router.
///
/// All routes are relative — the caller nests them under `/auth`.
pub fn build_router(svc: Arc<AuthService>) -> Router {
    let api = Router::new()
        .merge(users::routes())
        .merge(groups::routes())
        .merge(providers::routes())
        .merge(roles::routes())
        .merge(policies::routes())
        .merge(oauth::routes())
        .merge(me::routes())
        .merge(check::routes());

    Router::new()
        .nest("/auth", api)
        .layer(axum::middleware::from_fn_with_state(
            svc.clone(),
            middleware::auth_middleware,
        ))
        .with_state(svc)
}
