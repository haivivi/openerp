//! Search state — stored at `search/state`.

use flux_derive::state;
use serde::{Deserialize, Serialize};
use super::auth::UserProfile;
use super::timeline::FeedItem;

/// Search results page.
#[state("search/state")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchState {
    pub query: String,
    pub users: Vec<UserProfile>,
    pub tweets: Vec<FeedItem>,
    pub loading: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
