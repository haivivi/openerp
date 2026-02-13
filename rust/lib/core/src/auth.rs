//! Authentication trait for the DSL framework.
//!
//! The DSL framework does NOT depend on any specific auth module.
//! It only knows this trait. The concrete implementation is injected
//! at startup time.

use axum::http::HeaderMap;

use crate::ServiceError;

/// Pluggable authenticator. The DSL framework calls this for every
/// endpoint that has a `#[permission("...")]` annotation.
///
/// The check receives the request headers (for extracting tokens)
/// and the permission string from the DSL annotation.
pub trait Authenticator: Send + Sync + 'static {
    /// Authenticate a request and check the given permission.
    ///
    /// - `headers`: the HTTP request headers
    /// - `permission`: the string from `#[permission("module:resource:action")]`
    /// - Returns `Ok(())` if allowed, `Err(ServiceError)` if denied.
    fn check(
        &self,
        headers: &HeaderMap,
        permission: &str,
    ) -> Result<(), ServiceError>;
}

/// A no-op authenticator that allows everything. Used for testing
/// and for public-only APIs.
pub struct AllowAll;

impl Authenticator for AllowAll {
    fn check(&self, _headers: &HeaderMap, _permission: &str) -> Result<(), ServiceError> {
        Ok(())
    }
}

/// An authenticator that denies everything. Used for testing.
pub struct DenyAll;

impl Authenticator for DenyAll {
    fn check(&self, _headers: &HeaderMap, _permission: &str) -> Result<(), ServiceError> {
        Err(ServiceError::Validation("access denied".into()))
    }
}
