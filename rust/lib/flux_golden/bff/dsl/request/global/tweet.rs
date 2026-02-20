//! Tweet requests.

use flux_derive::request;

/// Create a new tweet (or reply).
#[request("tweet/create")]
pub struct CreateTweetReq {
    pub content: String,
    pub reply_to_id: Option<String>,
}

/// Like a tweet.
#[request("tweet/like")]
pub struct LikeTweetReq {
    pub tweet_id: String,
}

/// Unlike a tweet.
#[request("tweet/unlike")]
pub struct UnlikeTweetReq {
    pub tweet_id: String,
}

/// Load tweet detail with replies.
#[request("tweet/load")]
pub struct LoadTweetReq {
    pub tweet_id: String,
}
