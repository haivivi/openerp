//! Auth requests.
//!
//! Future macro form:
//! ```ignore
//! #[request("auth/login")]
//! pub struct LoginReq { ... }
//! ```

/// Login with username.
// #[request("auth/login")]
#[derive(Debug, Clone)]
pub struct LoginReq {
    pub username: String,
}

impl LoginReq {
    pub const PATH: &'static str = "auth/login";
}

/// Logout — clear session.
// #[request("auth/logout")]
#[derive(Debug, Clone)]
pub struct LogoutReq;

impl LogoutReq {
    pub const PATH: &'static str = "auth/logout";
}
