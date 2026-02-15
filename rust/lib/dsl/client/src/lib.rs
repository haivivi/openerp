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
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP {status}: {message}")]
    Server { status: u16, message: String },

    #[error("network: {0}")]
    Network(#[from] reqwest::Error),

    #[error("auth: {0}")]
    Auth(String),

    #[error("decode: {0}")]
    Decode(String),
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
            return Err(ApiError::Server { status: code, message: body });
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
            return Err(ApiError::Server { status: code, message: body });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
