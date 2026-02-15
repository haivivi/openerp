//! Auth requests.

use flux_derive::request;

/// Login with username.
#[request("auth/login")]
pub struct LoginReq {
    pub username: String,
}

/// Logout — clear session.
#[request("auth/logout")]
pub struct LogoutReq;
