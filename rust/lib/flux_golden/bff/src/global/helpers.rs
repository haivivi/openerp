//! Shared helpers for handlers.

use openerp_store::KvOps;

use crate::state::*;
use crate::server::model;

/// Convert a backend User to a BFF UserProfile.
pub fn user_to_profile(u: &model::User) -> UserProfile {
    UserProfile {
        id: u.id.to_string(),
        username: u.username.clone(),
        display_name: u.display_name.clone().unwrap_or_else(|| u.username.clone()),
        bio: u.bio.as_ref().map(|s| s.to_string()),
        avatar: u.avatar.as_ref().map(|s| s.to_string()),
        follower_count: u.follower_count,
        following_count: u.following_count,
        tweet_count: u.tweet_count,
    }
}

/// Convert a backend Tweet to a BFF FeedItem.
pub fn tweet_to_feed_item(
    t: &model::Tweet,
    current_user_id: &str,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) -> FeedItem {
    let author = users.get(&t.author_id)
        .ok()
        .flatten()
        .map(|u| user_to_profile(&u))
        .unwrap_or_else(|| UserProfile {
            id: t.author_id.to_string(),
            username: "unknown".into(),
            display_name: "Unknown".into(),
            bio: None, avatar: None,
            follower_count: 0, following_count: 0, tweet_count: 0,
        });

    let like_key = format!("{}:{}", current_user_id, t.id);
    let liked_by_me = likes.get(&like_key).ok().flatten().is_some();

    FeedItem {
        tweet_id: t.id.to_string(),
        author,
        content: t.content.clone(),
        like_count: t.like_count,
        liked_by_me,
        reply_count: t.reply_count,
        reply_to_id: t.reply_to_id.as_ref().map(|s| s.to_string()),
        created_at: t.created_at.to_string(),
    }
}

/// Build the home timeline (top-level tweets, newest first).
pub fn build_timeline(
    current_user_id: &str,
    tweets: &KvOps<model::Tweet>,
    users: &KvOps<model::User>,
    likes: &KvOps<model::Like>,
) -> TimelineFeed {
    let mut all = tweets.list().unwrap_or_default();
    all.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));

    let items: Vec<FeedItem> = all.iter()
        .filter(|t| t.reply_to_id.is_none())
        .map(|t| tweet_to_feed_item(t, current_user_id, users, likes))
        .collect();

    TimelineFeed { items, loading: false, has_more: false, error: None }
}
