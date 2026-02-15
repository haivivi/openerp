//! Profile page state — stored at `profile/{user_id}`.

use super::auth::UserProfile;
use super::timeline::FeedItem;

/// A user's profile page.
// #[state("profile/{user_id}")]
#[derive(Debug, Clone, PartialEq)]
pub struct ProfilePage {
    pub user: UserProfile,
    pub tweets: Vec<FeedItem>,
    pub followed_by_me: bool,
    pub loading: bool,
}

impl ProfilePage {
    /// Dynamic path: `profile/{user_id}`.
    pub fn path(user_id: &str) -> String {
        format!("profile/{}", user_id)
    }
}
