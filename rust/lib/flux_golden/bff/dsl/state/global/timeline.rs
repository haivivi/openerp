//! Timeline state — stored at `timeline/feed`.

use super::auth::UserProfile;

/// Home timeline feed.
// #[state("timeline/feed")]
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineFeed {
    pub items: Vec<FeedItem>,
    pub loading: bool,
    pub has_more: bool,
    pub error: Option<String>,
}

/// A single tweet rendered in a feed.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedItem {
    pub tweet_id: String,
    pub author: UserProfile,
    pub content: String,
    pub like_count: u32,
    pub liked_by_me: bool,
    pub reply_count: u32,
    pub reply_to_id: Option<String>,
    pub created_at: String,
}

impl TimelineFeed {
    pub const PATH: &'static str = "timeline/feed";
}
