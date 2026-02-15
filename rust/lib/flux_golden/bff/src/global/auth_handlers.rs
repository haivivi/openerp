//! Auth handler implementations.
//!
//! Future macro form:
//! ```ignore
//! #[flux_handlers]
//! impl TwitterHandlers {
//!     #[request("auth/login")]
//!     async fn login(&self, req: LoginReq, store: &FluxStore) { ... }
//! }
//! ```

use openerp_flux::StateStore;
use openerp_store::KvOps;

use crate::request::*;
use crate::state::*;
use crate::handlers::global::helpers;
use crate::server::model;

/// Handle `auth/login`.
pub async fn handle_login(
    req: &LoginReq,
    store: &StateStore,
    users: &KvOps<model::User>,
) {
    // Set busy.
    store.set(AuthState::PATH, AuthState {
        phase: AuthPhase::Unauthenticated,
        user: None,
        busy: true,
        error: None,
    });

    match users.get(&req.username) {
        Ok(Some(user)) => {
            let profile = helpers::user_to_profile(&user);
            store.set(AuthState::PATH, AuthState {
                phase: AuthPhase::Authenticated,
                user: Some(profile),
                busy: false,
                error: None,
            });
            store.set(AppRoute::PATH, AppRoute("/home".into()));
        }
        _ => {
            store.set(AuthState::PATH, AuthState {
                phase: AuthPhase::Unauthenticated,
                user: None,
                busy: false,
                error: Some(format!("User '{}' not found", req.username)),
            });
        }
    }
}

/// Handle `auth/logout`.
pub async fn handle_logout(store: &StateStore) {
    store.set(AuthState::PATH, AuthState {
        phase: AuthPhase::Unauthenticated,
        user: None,
        busy: false,
        error: None,
    });
    store.set(AppRoute::PATH, AppRoute("/login".into()));
    store.remove(TimelineFeed::PATH);
    store.remove(ComposeState::PATH);
}
