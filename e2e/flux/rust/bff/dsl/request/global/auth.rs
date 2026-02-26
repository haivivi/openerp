//! Auth requests.

use flux_derive::request;

/// Login with username + password.
#[request("auth/login")]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

/// Logout — clear session.
#[request("auth/logout")]
pub struct LogoutReq;
