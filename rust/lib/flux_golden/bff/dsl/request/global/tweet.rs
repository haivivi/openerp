//! Tweet requests.

/// Create a new tweet (or reply).
// #[request("tweet/create")]
#[derive(Debug, Clone)]
pub struct CreateTweetReq {
    pub content: String,
    pub reply_to_id: Option<String>,
}

impl CreateTweetReq {
    pub const PATH: &'static str = "tweet/create";
}

/// Like a tweet.
// #[request("tweet/like")]
#[derive(Debug, Clone)]
pub struct LikeTweetReq {
    pub tweet_id: String,
}

impl LikeTweetReq {
    pub const PATH: &'static str = "tweet/like";
}

/// Unlike a tweet.
// #[request("tweet/unlike")]
#[derive(Debug, Clone)]
pub struct UnlikeTweetReq {
    pub tweet_id: String,
}

impl UnlikeTweetReq {
    pub const PATH: &'static str = "tweet/unlike";
}

/// Load tweet detail with replies.
// #[request("tweet/load")]
#[derive(Debug, Clone)]
pub struct LoadTweetReq {
    pub tweet_id: String,
}

impl LoadTweetReq {
    pub const PATH: &'static str = "tweet/load";
}
