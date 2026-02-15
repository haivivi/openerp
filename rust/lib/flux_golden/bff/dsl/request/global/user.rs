//! User requests.

/// Follow a user.
// #[request("user/follow")]
#[derive(Debug, Clone)]
pub struct FollowUserReq {
    pub user_id: String,
}

impl FollowUserReq {
    pub const PATH: &'static str = "user/follow";
}

/// Unfollow a user.
// #[request("user/unfollow")]
#[derive(Debug, Clone)]
pub struct UnfollowUserReq {
    pub user_id: String,
}

impl UnfollowUserReq {
    pub const PATH: &'static str = "user/unfollow";
}

/// Load a user's profile page.
// #[request("profile/load")]
#[derive(Debug, Clone)]
pub struct LoadProfileReq {
    pub user_id: String,
}

impl LoadProfileReq {
    pub const PATH: &'static str = "profile/load";
}
