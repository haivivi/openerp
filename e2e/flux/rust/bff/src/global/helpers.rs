//! Shared helpers for handlers.
//! Now uses HTTP client instead of direct KvOps.

use openerp_client::ResourceClient;

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
    users: &[model::User],
    likes: &[model::Like],
) -> FeedItem {
    let author_id = t.author.resource_id();
    let author = users.iter()
        .find(|u| u.id.as_str() == author_id)
        .map(|u| user_to_profile(u))
        .unwrap_or_else(|| UserProfile {
            id: author_id.to_string(),
            username: "unknown".into(),
            display_name: "Unknown".into(),
            bio: None, avatar: None,
            follower_count: 0, following_count: 0, tweet_count: 0,
        });

    let like_key = format!("{}:{}", current_user_id, t.id);
    let liked_by_me = likes.iter().any(|l| l.id.as_str() == like_key);

    FeedItem {
        tweet_id: t.id.to_string(),
        author,
        content: t.content.clone(),
        like_count: t.like_count,
        liked_by_me,
        reply_count: t.reply_count,
        reply_to_id: t.reply_to.as_ref().map(|n| n.resource_id().to_string()),
        created_at: t.created_at.to_string(),
    }
}

/// Build timeline from fetched data.
pub fn build_timeline(
    current_user_id: &str,
    tweets: &mut Vec<model::Tweet>,
    users: &[model::User],
    likes: &[model::Like],
) -> TimelineFeed {
    tweets.sort_by(|a, b| b.created_at.as_str().cmp(a.created_at.as_str()));

    let items: Vec<FeedItem> = tweets.iter()
        .filter(|t| t.reply_to.is_none())
        .map(|t| tweet_to_feed_item(t, current_user_id, users, likes))
        .collect();

    TimelineFeed { items, loading: false, has_more: false, error: None }
}
