//! Search requests.

use flux_derive::request;

/// Search users and tweets by keyword.
#[request("search/query")]
pub struct SearchReq {
    pub query: String,
}

impl SearchReq {
    pub const CLEAR_PATH: &'static str = "search/clear";
}

/// Clear search results.
#[request("search/clear")]
pub struct SearchClearReq;
