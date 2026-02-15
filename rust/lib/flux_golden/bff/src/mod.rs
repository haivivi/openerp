//! BFF handler implementations and Flux wiring.
//!
//! `register_handlers` is the code that `#[flux_handlers]` macro will generate.
//! It takes each `#[request("path")]`-annotated method and registers it
//! with the Flux router, wiring the typed payload downcast + store access.

pub mod global;

use std::sync::Arc;

use openerp_flux::{Flux, StateStore};
use openerp_store::KvOps;

use crate::server::model;
use crate::request::*;
use crate::state::*;
use self::global::{app_handlers, auth_handlers, tweet_handlers, user_handlers, helpers};

/// Backend context — holds KvOps for all resources.
/// This is what the handler struct's `&self` provides.
pub struct TwitterContext {
    pub users: KvOps<model::User>,
    pub tweets: KvOps<model::Tweet>,
    pub likes: KvOps<model::Like>,
    pub follows: KvOps<model::Follow>,
}

/// Register all handlers with a Flux instance.
///
/// THIS IS THE CODE THE MACRO GENERATES:
/// For each `#[request("path")] fn method(...)`, it generates a
/// `flux.on("path", ...)` call that downcasts the payload and calls the handler.
pub fn register_handlers(flux: &Flux, ctx: Arc<TwitterContext>) {
    // app/initialize
    flux.on(InitializeReq::PATH, |_, _, store: Arc<StateStore>| async move {
        app_handlers::handle_initialize(&store).await;
    });

    // auth/login
    {
        let ctx = ctx.clone();
        flux.on(LoginReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<LoginReq>().unwrap();
                auth_handlers::handle_login(req, &store, &ctx.users).await;
                // Auto-load timeline after successful login.
                let auth = store.get(AuthState::PATH)
                    .and_then(|v| v.downcast_ref::<AuthState>().cloned());
                if let Some(auth) = auth {
                    if auth.phase == AuthPhase::Authenticated {
                        let uid = auth.user.as_ref().map(|u| u.id.as_str()).unwrap_or("");
                        let feed = helpers::build_timeline(uid, &ctx.tweets, &ctx.users, &ctx.likes);
                        store.set(TimelineFeed::PATH, feed);
                    }
                }
            }
        });
    }

    // auth/logout
    flux.on(LogoutReq::PATH, |_, _, store: Arc<StateStore>| async move {
        auth_handlers::handle_logout(&store).await;
    });

    // timeline/load
    {
        let ctx = ctx.clone();
        flux.on(TimelineLoadReq::PATH, move |_, _, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                app_handlers::handle_timeline_load(&store, &ctx.tweets, &ctx.users, &ctx.likes).await;
            }
        });
    }

    // tweet/create
    {
        let ctx = ctx.clone();
        flux.on(CreateTweetReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<CreateTweetReq>().unwrap();
                tweet_handlers::handle_create(req, &store, &ctx.tweets, &ctx.users, &ctx.likes).await;
            }
        });
    }

    // tweet/like
    {
        let ctx = ctx.clone();
        flux.on(LikeTweetReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<LikeTweetReq>().unwrap();
                tweet_handlers::handle_like(req, &store, &ctx.tweets, &ctx.users, &ctx.likes).await;
            }
        });
    }

    // tweet/unlike
    {
        let ctx = ctx.clone();
        flux.on(UnlikeTweetReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<UnlikeTweetReq>().unwrap();
                tweet_handlers::handle_unlike(req, &store, &ctx.tweets, &ctx.users, &ctx.likes).await;
            }
        });
    }

    // tweet/load
    {
        let ctx = ctx.clone();
        flux.on(LoadTweetReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<LoadTweetReq>().unwrap();
                tweet_handlers::handle_load(req, &store, &ctx.tweets, &ctx.users, &ctx.likes).await;
            }
        });
    }

    // user/follow
    {
        let ctx = ctx.clone();
        flux.on(FollowUserReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<FollowUserReq>().unwrap();
                user_handlers::handle_follow(req, &store, &ctx.users, &ctx.follows).await;
            }
        });
    }

    // user/unfollow
    {
        let ctx = ctx.clone();
        flux.on(UnfollowUserReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<UnfollowUserReq>().unwrap();
                user_handlers::handle_unfollow(req, &store, &ctx.users, &ctx.follows).await;
            }
        });
    }

    // profile/load
    {
        let ctx = ctx.clone();
        flux.on(LoadProfileReq::PATH, move |_, payload, store: Arc<StateStore>| {
            let ctx = ctx.clone();
            async move {
                let req = payload.downcast_ref::<LoadProfileReq>().unwrap();
                user_handlers::handle_load_profile(
                    req, &store, &ctx.users, &ctx.tweets, &ctx.likes, &ctx.follows,
                ).await;
            }
        });
    }

    // compose/update-field
    flux.on(ComposeUpdateReq::PATH, |_, payload, store: Arc<StateStore>| async move {
        let req = payload.downcast_ref::<ComposeUpdateReq>().unwrap();
        app_handlers::handle_compose_update(req, &store).await;
    });
}
