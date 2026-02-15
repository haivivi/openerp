//! User handler implementations (follow, unfollow, profile).

use openerp_flux::StateStore;
use openerp_store::KvOps;
use openerp_types::*;

use crate::request::*;
use crate::state::*;
use crate::handlers::global::helpers;
use crate::server::model;

fn current_user_id(store: &StateStore) -> String {
    store.get(AuthState::PATH)
        .and_then(|v| v.downcast_ref::<AuthState>()
            .and_then(|a| a.user.as_ref().map(|u| u.id.clone())))
        .unwrap_or_default()
}

/// Handle `user/follow`.
pub async fn handle_follow(
    req: &FollowUserReq,
    store: &StateStore,
    users: &KvOps<model::User>,
    follows: &KvOps<model::Follow>,
) {
    let uid = current_user_id(store);

    let follow = model::Follow {
        id: Id::default(),
        follower_id: Id::new(&uid),
        followee_id: Id::new(&req.user_id),
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(),
    };

    if follows.save_new(follow).is_ok() {
        if let Ok(Some(mut me)) = users.get(&uid) {
            me.following_count += 1;
            let _ = users.save(me);
        }
        if let Ok(Some(mut them)) = users.get(&req.user_id) {
            them.follower_count += 1;
            let _ = users.save(them);
        }
        // Update auth state to reflect new following count.
        refresh_auth_profile(store, users, &uid);
    }
}

/// Handle `user/unfollow`.
pub async fn handle_unfollow(
    req: &UnfollowUserReq,
    store: &StateStore,
    users: &KvOps<model::User>,
    follows: &KvOps<model::Follow>,
) {
    let uid = current_user_id(store);
    let key = format!("{}:{}", uid, req.user_id);

    if follows.delete(&key).is_ok() {
        if let Ok(Some(mut me)) = users.get(&uid) {
            me.following_count = me.following_count.saturating_sub(1);
            let _ = users.save(me);
        }
        if let Ok(Some(mut them)) = users.get(&req.user_id) {
            them.follower_count = them.follower_count.saturating_sub(1);
            let _ = users.save(them);
        }
        refresh_auth_profile(store, users, &uid);
    }
}

/// Handle `profile/load`.
pub async fn handle_load_profile(
    req: &LoadProfileReq,
    store: &StateStore,
    users: &KvOps<model::User>,
    tweets: &KvOps<model::Tweet>,
    likes: &KvOps<model::Like>,
    follows: &KvOps<model::Follow>,
) {
    let uid = current_user_id(store);

    if let Ok(Some(user)) = users.get(&req.user_id) {
        let profile = helpers::user_to_profile(&user);

        // Load user's tweets.
        let mut user_tweets: Vec<model::Tweet> = tweets.list().unwrap_or_default()
            .into_iter()
            .filter(|t| t.author_id.as_str() == req.user_id)
            .collect();
        user_tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));
        let tweet_items: Vec<FeedItem> = user_tweets.iter()
            .map(|t| helpers::tweet_to_feed_item(t, &uid, users, likes))
            .collect();

        // Check if current user follows this user.
        let follow_key = format!("{}:{}", uid, req.user_id);
        let followed_by_me = follows.get(&follow_key).ok().flatten().is_some();

        let path = ProfilePage::path(&req.user_id);
        store.set(&path, ProfilePage {
            user: profile,
            tweets: tweet_items,
            followed_by_me,
            loading: false,
        });
        store.set(AppRoute::PATH, AppRoute(format!("/profile/{}", req.user_id)));
    }
}

/// Refresh the auth/state with latest user profile from backend.
fn refresh_auth_profile(
    store: &StateStore,
    users: &KvOps<model::User>,
    uid: &str,
) {
    if let Ok(Some(me)) = users.get(uid) {
        let profile = helpers::user_to_profile(&me);
        store.set(AuthState::PATH, AuthState {
            phase: AuthPhase::Authenticated,
            user: Some(profile),
            busy: false,
            error: None,
        });
    }
}
