//! OpenERP generated HTTP client.
//!
//! Provides a type-safe client for `#[model]` resources. Authentication
//! is handled by pluggable [`TokenSource`] implementations (Go-style
//! `oauth2.TokenSource` pattern).
//!
//! # Usage
//!
//! ```ignore
//! use openerp_client::{ResourceClient, PasswordLogin};
//!
//! let ts = PasswordLogin::new("http://localhost:8080", "root", "secret");
//! let client = ResourceClient::<User>::new("http://localhost:8080", Arc::new(ts));
//! let users = client.list().await?;
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use openerp_types::DslModel;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// ── Error ───────────────────────────────────────────────────────────

/// Client-side API error.
///
/// `Server` errors carry the HTTP status code and parsed error message
/// from the server's `{"error": "..."}` JSON body.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Server returned a non-2xx response.
    ///
    /// - `status`: HTTP status code (e.g. 400, 404, 500)
    /// - `message`: parsed from server JSON `{"error": "..."}`, or raw body
    ///   if not JSON
    #[error("HTTP {status}: {message}")]
    Server { status: u16, message: String },

    /// Network-level failure (DNS, connection refused, timeout, etc.).
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),

    /// Authentication failure (login rejected, token source error).
    #[error("auth: {0}")]
    Auth(String),

    /// Response body could not be deserialized.
    #[error("decode: {0}")]
    Decode(String),
}

impl ApiError {
    /// HTTP status code, if this is a server error.
    pub fn status(&self) -> Option<u16> {
        match self {
            ApiError::Server { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The error message (parsed from server JSON or variant message).
    pub fn message(&self) -> &str {
        match self {
            ApiError::Server { message, .. } => message,
            ApiError::Network(e) => return Box::leak(e.to_string().into_boxed_str()),
            ApiError::Auth(msg) => msg,
            ApiError::Decode(msg) => msg,
        }
    }

    /// True if the server returned 404 Not Found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ApiError::Server { status: 404, .. })
    }

    /// True if the server returned 400 Bad Request (validation / auth).
    pub fn is_bad_request(&self) -> bool {
        matches!(self, ApiError::Server { status: 400, .. })
    }

    /// True if the server returned 401 Unauthorized.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, ApiError::Server { status: 401, .. })
    }

    /// True if the server returned 403 Forbidden.
    pub fn is_forbidden(&self) -> bool {
        matches!(self, ApiError::Server { status: 403, .. })
    }

    /// True if the server returned 409 Conflict.
    pub fn is_conflict(&self) -> bool {
        matches!(self, ApiError::Server { status: 409, .. })
    }

    /// True if this is any authentication-related error
    /// (login failure or token source error).
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ApiError::Auth(_))
    }

    /// True if this is a network-level error (not a server response).
    pub fn is_network_error(&self) -> bool {
        matches!(self, ApiError::Network(_))
    }

    /// Parse a server error body. Extracts `{"error": "..."}` if JSON,
    /// otherwise uses raw text.
    fn from_response_body(status: u16, body: &str) -> Self {
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("error")?.as_str().map(String::from))
            .unwrap_or_else(|| body.to_string());
        ApiError::Server { status, message }
    }
}

// ── TokenSource ─────────────────────────────────────────────────────

/// Pluggable token provider. Called before every API request.
///
/// Implementations handle token acquisition, caching, and refresh.
/// Returns `Ok(None)` to skip the Authorization header (anonymous).
#[async_trait::async_trait]
pub trait TokenSource: Send + Sync + 'static {
    async fn token(&self) -> Result<Option<String>, ApiError>;
}

/// No authentication — anonymous requests.
pub struct NoAuth;

#[async_trait::async_trait]
impl TokenSource for NoAuth {
    async fn token(&self) -> Result<Option<String>, ApiError> {
        Ok(None)
    }
}

/// Static bearer token (already obtained externally).
pub struct StaticToken(String);

impl StaticToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

#[async_trait::async_trait]
impl TokenSource for StaticToken {
    async fn token(&self) -> Result<Option<String>, ApiError> {
        Ok(Some(self.0.clone()))
    }
}

/// Password-based login. Lazily authenticates on first use, caches the
/// JWT, and re-authenticates when the token expires.
pub struct PasswordLogin {
    http: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
    cached: tokio::sync::RwLock<Option<CachedToken>>,
}

struct CachedToken {
    access_token: String,
    /// Absolute expiry timestamp (seconds since epoch).
    expires_at: i64,
}

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
    expires_in: u64,
}

impl PasswordLogin {
    pub fn new(base_url: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            username: username.into(),
            password: password.into(),
            cached: tokio::sync::RwLock::new(None),
        }
    }

    async fn do_login(&self) -> Result<CachedToken, ApiError> {
        let url = format!("{}/auth/login", self.base_url);
        let resp = self.http.post(&url)
            .json(&serde_json::json!({
                "username": self.username,
                "password": self.password,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Auth(format!("login failed ({}): {}", status, body)));
        }

        let lr: LoginResponse = resp.json().await
            .map_err(|e| ApiError::Decode(format!("login response: {}", e)))?;

        let now = chrono::Utc::now().timestamp();
        // Expire 30s early to avoid edge-case races.
        let expires_at = now + lr.expires_in as i64 - 30;

        Ok(CachedToken {
            access_token: lr.access_token,
            expires_at,
        })
    }
}

#[async_trait::async_trait]
impl TokenSource for PasswordLogin {
    async fn token(&self) -> Result<Option<String>, ApiError> {
        // Fast path: read lock, check cache.
        {
            let guard = self.cached.read().await;
            if let Some(ref cached) = *guard {
                let now = chrono::Utc::now().timestamp();
                if now < cached.expires_at {
                    return Ok(Some(cached.access_token.clone()));
                }
            }
        }

        // Slow path: write lock, re-check, login.
        let mut guard = self.cached.write().await;
        // Double-check after acquiring write lock.
        if let Some(ref cached) = *guard {
            let now = chrono::Utc::now().timestamp();
            if now < cached.expires_at {
                return Ok(Some(cached.access_token.clone()));
            }
        }

        let fresh = self.do_login().await?;
        let token = fresh.access_token.clone();
        *guard = Some(fresh);
        Ok(Some(token))
    }
}

// ── List response ───────────────────────────────────────────────────

/// Server list response (matches `ListResult<T>` on the server side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
}

// ── ResourceClient ──────────────────────────────────────────────────

/// Type-safe CRUD client for a single `#[model]` resource.
///
/// API path is derived from `T::module()` and `T::resource_path()`:
/// `{base_url}/admin/{module}/{resource_path}`.
pub struct ResourceClient<T: DslModel> {
    http: reqwest::Client,
    base_url: String,
    token_source: Arc<dyn TokenSource>,
    _phantom: PhantomData<T>,
}

impl<T: DslModel> ResourceClient<T> {
    pub fn new(base_url: impl Into<String>, token_source: Arc<dyn TokenSource>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token_source,
            _phantom: PhantomData,
        }
    }

    /// Base URL for this resource: `/admin/{module}/{path}`.
    fn collection_url(&self) -> String {
        format!("{}/admin/{}/{}", self.base_url, T::module(), T::resource_path())
    }

    /// URL for a single item: `/admin/{module}/{path}/{id}`.
    fn item_url(&self, id: &str) -> String {
        format!("{}/{}", self.collection_url(), id)
    }

    /// Build a request with auth header.
    async fn authed(&self, builder: reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder, ApiError> {
        match self.token_source.token().await? {
            Some(token) => Ok(builder.bearer_auth(token)),
            None => Ok(builder),
        }
    }

    /// Parse an API response, mapping HTTP errors to `ApiError`.
    async fn parse<R: DeserializeOwned>(resp: reqwest::Response) -> Result<R, ApiError> {
        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::from_response_body(code, &body));
        }
        resp.json::<R>().await
            .map_err(|e| ApiError::Decode(format!("response body: {}", e)))
    }

    /// List all records.
    pub async fn list(&self) -> Result<ListResponse<T>, ApiError> {
        let req = self.http.get(&self.collection_url());
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse(resp).await
    }

    /// Get a record by ID.
    pub async fn get(&self, id: &str) -> Result<T, ApiError> {
        let req = self.http.get(&self.item_url(id));
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse(resp).await
    }

    /// Create a new record.
    pub async fn create(&self, item: &T) -> Result<T, ApiError> {
        let req = self.http.post(&self.collection_url()).json(item);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse(resp).await
    }

    /// Update an existing record by ID.
    pub async fn update(&self, id: &str, item: &T) -> Result<T, ApiError> {
        let req = self.http.put(&self.item_url(id)).json(item);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse(resp).await
    }

    /// Delete a record by ID.
    pub async fn delete(&self, id: &str) -> Result<(), ApiError> {
        let req = self.http.delete(&self.item_url(id));
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::from_response_body(code, &body));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TokenSource ─────────────────────────────────────────────────

    #[tokio::test]
    async fn no_auth_returns_none() {
        let ts = NoAuth;
        assert!(ts.token().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn static_token_returns_value() {
        let ts = StaticToken::new("my-jwt-token");
        assert_eq!(ts.token().await.unwrap(), Some("my-jwt-token".to_string()));
    }

    // ── ApiError construction + helpers ──────────────────────────────

    #[test]
    fn from_response_body_parses_json_error() {
        let err = ApiError::from_response_body(404, r#"{"error":"id 'x123' not found"}"#);
        assert_eq!(err.status(), Some(404));
        assert_eq!(err.message(), "id 'x123' not found");
        assert!(err.is_not_found());
        assert!(!err.is_bad_request());
    }

    #[test]
    fn from_response_body_falls_back_to_raw_text() {
        let err = ApiError::from_response_body(500, "Internal Server Error");
        assert_eq!(err.status(), Some(500));
        assert_eq!(err.message(), "Internal Server Error");
        assert!(!err.is_not_found());
    }

    #[test]
    fn from_response_body_handles_empty_body() {
        let err = ApiError::from_response_body(502, "");
        assert_eq!(err.status(), Some(502));
        assert_eq!(err.message(), "");
    }

    #[test]
    fn from_response_body_handles_non_error_json() {
        // JSON that doesn't have an "error" field → raw body.
        let err = ApiError::from_response_body(400, r#"{"detail":"bad"}"#);
        assert_eq!(err.message(), r#"{"detail":"bad"}"#);
    }

    #[test]
    fn status_helpers_cover_all_codes() {
        assert!(ApiError::from_response_body(400, "").is_bad_request());
        assert!(ApiError::from_response_body(401, "").is_unauthorized());
        assert!(ApiError::from_response_body(403, "").is_forbidden());
        assert!(ApiError::from_response_body(404, "").is_not_found());
        assert!(ApiError::from_response_body(409, "").is_conflict());

        // Negative cases: 404 is NOT bad_request.
        assert!(!ApiError::from_response_body(404, "").is_bad_request());
        assert!(!ApiError::from_response_body(400, "").is_not_found());
    }

    #[test]
    fn auth_error_helpers() {
        let err = ApiError::Auth("login failed".into());
        assert!(err.is_auth_error());
        assert!(!err.is_network_error());
        assert_eq!(err.status(), None);
        assert_eq!(err.message(), "login failed");
    }

    #[test]
    fn display_format_is_human_readable() {
        let err = ApiError::from_response_body(404, r#"{"error":"id 'abc' not found"}"#);
        let display = format!("{}", err);
        assert_eq!(display, "HTTP 404: id 'abc' not found");

        let err = ApiError::Auth("login failed (401): invalid credentials".into());
        let display = format!("{}", err);
        assert_eq!(display, "auth: login failed (401): invalid credentials");

        let err = ApiError::Decode("expected JSON".into());
        let display = format!("{}", err);
        assert_eq!(display, "decode: expected JSON");
    }
}
