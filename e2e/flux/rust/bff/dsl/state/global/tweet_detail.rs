//! Tweet detail state — stored at `tweet/{tweet_id}`.

use flux_derive::state;
use serde::{Deserialize, Serialize};
use super::timeline::FeedItem;

/// Tweet detail view with replies.
#[state("tweet")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TweetDetail {
    pub tweet: FeedItem,
    pub replies: Vec<FeedItem>,
    pub loading: bool,
}

impl TweetDetail {
    pub fn path(tweet_id: &str) -> String {
        format!("tweet/{}", tweet_id)
    }
}
