//! Tweet handler implementations.

use openerp_flux::StateStore;
use openerp_store::KvOps;
use openerp_types::*;

use crate::request::*;
use crate::state::*;
use crate::handlers::global::helpers;
use crate::server::model;

/// Helper: get current user ID from auth state.
fn current_user_id(store: &StateStore) -> String {
    store.get(AuthState::PATH)
        .and_then(|v| v.downcast_ref::<AuthState>()
            .and_then(|a| a.user.as_ref().map(|u| u.id.clone())))
        .unwrap_or_default()
}

/// Handle `tweet/create`.
pub async fn handle_create(
    req: &CreateTweetReq,
    store: &StateStore,
    tweets: &KvOps<model::Tweet>,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) {
    let uid = current_user_id(store);

    // Validate content.
    if req.content.trim().is_empty() {
        store.set(ComposeState::PATH, ComposeState {
            content: req.content.clone(),
            reply_to_id: req.reply_to_id.clone(),
            busy: false,
            error: Some("Tweet cannot be empty".into()),
        });
        return;
    }
    if req.content.len() > 280 {
        store.set(ComposeState::PATH, ComposeState {
            content: req.content.clone(),
            reply_to_id: req.reply_to_id.clone(),
            busy: false,
            error: Some("Tweet exceeds 280 characters".into()),
        });
        return;
    }

    // Set busy.
    store.set(ComposeState::PATH, ComposeState {
        content: req.content.clone(),
        reply_to_id: req.reply_to_id.clone(),
        busy: true,
        error: None,
    });

    let tweet = model::Tweet {
        id: Id::default(),
        author_id: Id::new(&uid),
        content: req.content.clone(),
        like_count: 0,
        reply_count: 0,
        reply_to_id: req.reply_to_id.as_ref().map(|s| Id::new(s)),
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(),
    };

    match tweets.save_new(tweet) {
        Ok(_) => {
            // Increment author tweet count.
            if let Ok(Some(mut user)) = users.get(&uid) {
                user.tweet_count += 1;
                let _ = users.save(user);
            }
            // Increment parent reply count if this is a reply.
            if let Some(ref parent_id) = req.reply_to_id {
                if let Ok(Some(mut parent)) = tweets.get(parent_id) {
                    parent.reply_count += 1;
                    let _ = tweets.save(parent);
                }
            }
            // Clear compose, refresh timeline.
            store.set(ComposeState::PATH, ComposeState::empty());
            store.set(TimelineFeed::PATH, helpers::build_timeline(&uid, tweets, users, likes));
        }
        Err(e) => {
            store.set(ComposeState::PATH, ComposeState {
                content: req.content.clone(),
                reply_to_id: req.reply_to_id.clone(),
                busy: false,
                error: Some(e.to_string()),
            });
        }
    }
}

/// Handle `tweet/like`.
pub async fn handle_like(
    req: &LikeTweetReq,
    store: &StateStore,
    tweets: &KvOps<model::Tweet>,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) {
    let uid = current_user_id(store);

    let like = model::Like {
        id: Id::default(),
        user_id: Id::new(&uid),
        tweet_id: Id::new(&req.tweet_id),
        display_name: None, description: None, metadata: None,
        created_at: DateTime::default(), updated_at: DateTime::default(),
    };

    if likes.save_new(like).is_ok() {
        if let Ok(Some(mut tweet)) = tweets.get(&req.tweet_id) {
            tweet.like_count += 1;
            let _ = tweets.save(tweet);
        }
        store.set(TimelineFeed::PATH, helpers::build_timeline(&uid, tweets, users, likes));
    }
}

/// Handle `tweet/unlike`.
pub async fn handle_unlike(
    req: &UnlikeTweetReq,
    store: &StateStore,
    tweets: &KvOps<model::Tweet>,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) {
    let uid = current_user_id(store);
    let like_key = format!("{}:{}", uid, req.tweet_id);

    if likes.delete(&like_key).is_ok() {
        if let Ok(Some(mut tweet)) = tweets.get(&req.tweet_id) {
            tweet.like_count = tweet.like_count.saturating_sub(1);
            let _ = tweets.save(tweet);
        }
        store.set(TimelineFeed::PATH, helpers::build_timeline(&uid, tweets, users, likes));
    }
}

/// Handle `tweet/load` — load tweet detail with replies.
pub async fn handle_load(
    req: &LoadTweetReq,
    store: &StateStore,
    tweets: &KvOps<model::Tweet>,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) {
    let uid = current_user_id(store);

    if let Ok(Some(tweet)) = tweets.get(&req.tweet_id) {
        let item = helpers::tweet_to_feed_item(&tweet, &uid, users, likes);

        // Load replies.
        let mut replies: Vec<model::Tweet> = tweets.list().unwrap_or_default()
            .into_iter()
            .filter(|t| t.reply_to_id.as_ref().map(|s| s.as_str()) == Some(&req.tweet_id))
            .collect();
        replies.sort_by(|a, b| a.created_at.as_str().cmp(b.created_at.as_str()));
        let reply_items: Vec<FeedItem> = replies.iter()
            .map(|t| helpers::tweet_to_feed_item(t, &uid, users, likes))
            .collect();

        let path = TweetDetail::path(&req.tweet_id);
        store.set(&path, TweetDetail { tweet: item, replies: reply_items, loading: false });
        store.set(AppRoute::PATH, AppRoute(format!("/tweet/{}", req.tweet_id)));
    }
}
