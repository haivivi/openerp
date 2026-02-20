//! Auth state — stored at `auth/state`.

use flux_derive::state;
use serde::{Deserialize, Serialize};

/// Authentication state — the UI reads this to decide what to show.
#[state("auth/state")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    pub phase: AuthPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserProfile>,
    pub busy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthPhase {
    Unauthenticated,
    Authenticated,
}

/// Logged-in user's profile summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    pub follower_count: u32,
    pub following_count: u32,
    pub tweet_count: u32,
}
