//! User requests.

use flux_derive::request;

/// Follow a user.
#[request("user/follow")]
pub struct FollowUserReq {
    pub user_id: String,
}

/// Unfollow a user.
#[request("user/unfollow")]
pub struct UnfollowUserReq {
    pub user_id: String,
}

/// Load a user's profile page.
#[request("profile/load")]
pub struct LoadProfileReq {
    pub user_id: String,
}
