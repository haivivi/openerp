//! Tweet detail state — stored at `tweet/{tweet_id}`.

use super::timeline::FeedItem;

/// Tweet detail view with replies.
// #[state("tweet/{tweet_id}")]
#[derive(Debug, Clone, PartialEq)]
pub struct TweetDetail {
    pub tweet: FeedItem,
    pub replies: Vec<FeedItem>,
    pub loading: bool,
}

impl TweetDetail {
    /// Dynamic path: `tweet/{tweet_id}`.
    pub fn path(tweet_id: &str) -> String {
        format!("tweet/{}", tweet_id)
    }
}
