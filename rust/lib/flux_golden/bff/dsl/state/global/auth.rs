//! Auth state — stored at `auth/state`.

use flux_derive::state;

/// Authentication state — the UI reads this to decide what to show.
#[state("auth/state")]
pub struct AuthState {
    pub phase: AuthPhase,
    pub user: Option<UserProfile>,
    pub busy: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthPhase {
    Unauthenticated,
    Authenticated,
}

/// Logged-in user's profile summary.
#[derive(Debug, Clone, PartialEq)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub follower_count: u32,
    pub following_count: u32,
    pub tweet_count: u32,
}
