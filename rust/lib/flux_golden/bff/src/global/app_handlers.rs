//! App lifecycle handler implementations.

use openerp_flux::StateStore;
use openerp_store::KvOps;

use crate::request::*;
use crate::state::*;
use crate::handlers::global::helpers;
use crate::server::model;

/// Handle `app/initialize`.
pub async fn handle_initialize(store: &StateStore) {
    store.set(AuthState::PATH, AuthState {
        phase: AuthPhase::Unauthenticated,
        user: None,
        busy: false,
        error: None,
    });
    store.set(AppRoute::PATH, AppRoute("/login".into()));
}

/// Handle `timeline/load`.
pub async fn handle_timeline_load(
    store: &StateStore,
    tweets: &KvOps<model::Tweet>,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) {
    let uid = store.get(AuthState::PATH)
        .and_then(|v| v.downcast_ref::<AuthState>()
            .and_then(|a| a.user.as_ref().map(|u| u.id.clone())))
        .unwrap_or_default();

    store.set(TimelineFeed::PATH, TimelineFeed {
        items: vec![], loading: true, has_more: false, error: None,
    });
    store.set(TimelineFeed::PATH, helpers::build_timeline(&uid, tweets, users, likes));
}

/// Handle `compose/update-field`.
pub async fn handle_compose_update(req: &ComposeUpdateReq, store: &StateStore) {
    let mut state = store.get(ComposeState::PATH)
        .and_then(|v| v.downcast_ref::<ComposeState>().cloned())
        .unwrap_or_else(ComposeState::empty);

    match req.field.as_str() {
        "content" => state.content = req.value.clone(),
        _ => {}
    }
    state.error = None; // Clear error on edit.
    store.set(ComposeState::PATH, state);
}
