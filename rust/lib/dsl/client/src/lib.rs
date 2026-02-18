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

use openerp_types::{DslModel, Format, FromFlatBuffer, FromFlatBufferList, MIME_FLATBUFFERS};
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
    /// - `code`: stable error code from server (e.g. "NOT_FOUND", "ALREADY_EXISTS")
    /// - `message`: human-readable error message
    #[error("HTTP {status} [{code}]: {message}")]
    Server { status: u16, code: String, message: String },

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

    /// Stable error code from the server (e.g. "NOT_FOUND", "ALREADY_EXISTS").
    ///
    /// Returns `None` for non-server errors (Auth, Network, Decode).
    pub fn error_code(&self) -> Option<&str> {
        match self {
            ApiError::Server { code, .. } => Some(code),
            _ => None,
        }
    }

    /// The human-readable error message.
    pub fn message(&self) -> &str {
        match self {
            ApiError::Server { message, .. } => message,
            ApiError::Auth(msg) => msg,
            ApiError::Decode(msg) => msg,
            ApiError::Network(_) => "network error",
        }
    }

    // ── Code-based helpers (stable — match on error code, not HTTP status) ──

    /// True if the server returned NOT_FOUND.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "NOT_FOUND")
    }

    /// True if the server returned ALREADY_EXISTS.
    pub fn is_already_exists(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "ALREADY_EXISTS")
    }

    /// True if the server returned VALIDATION_FAILED.
    pub fn is_validation_failed(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "VALIDATION_FAILED")
    }

    /// True if the server returned UNAUTHENTICATED (missing/invalid token).
    pub fn is_unauthenticated(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "UNAUTHENTICATED")
    }

    /// True if the server returned PERMISSION_DENIED.
    pub fn is_permission_denied(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "PERMISSION_DENIED")
    }

    /// True if the server returned READ_ONLY.
    pub fn is_read_only(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "READ_ONLY")
    }

    /// True if the server returned CONFLICT (optimistic locking failure).
    pub fn is_conflict(&self) -> bool {
        matches!(self, ApiError::Server { code, .. } if code == "CONFLICT")
    }

    /// True if this is a client-side authentication error
    /// (login failure / token source error — not a server 401).
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ApiError::Auth(_))
    }

    /// True if this is a network-level error (not a server response).
    pub fn is_network_error(&self) -> bool {
        matches!(self, ApiError::Network(_))
    }

    /// Parse a server error body.
    ///
    /// Expected format: `{"code": "NOT_FOUND", "message": "..."}`
    /// Falls back to legacy `{"error": "..."}` or raw text.
    fn from_response_body(status: u16, body: &str) -> Self {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            let code = v.get("code")
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();
            let message = v.get("message")
                .or_else(|| v.get("error"))
                .and_then(|m| m.as_str())
                .unwrap_or(body)
                .to_string();
            return ApiError::Server { status, code, message };
        }
        ApiError::Server {
            status,
            code: "UNKNOWN".to_string(),
            message: body.to_string(),
        }
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

// ── List / Count responses ──────────────────────────────────────────

/// Server list response (matches `ListResult<T>` on the server side).
///
/// Uses `has_more` instead of `total` — total count is a separate
/// concern available via [`ResourceClient::count()`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

/// Server count response from the `@count` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResponse {
    pub count: usize,
}

/// Parameters for paginated list requests.
#[derive(Debug, Clone, Default)]
pub struct ListParams {
    /// Maximum number of items to return. Server default: 50.
    pub limit: Option<usize>,
    /// Number of items to skip. Default: 0.
    pub offset: Option<usize>,
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

    /// List records with optional pagination.
    ///
    /// ```ignore
    /// // Default (server decides limit, offset=0):
    /// let page = client.list(None).await?;
    ///
    /// // Explicit pagination:
    /// let page = client.list(Some(&ListParams { limit: Some(20), offset: Some(40) })).await?;
    /// while page.has_more { ... }
    /// ```
    pub async fn list(&self, params: Option<&ListParams>) -> Result<ListResponse<T>, ApiError> {
        let mut url = self.collection_url();
        if let Some(p) = params {
            let mut parts = Vec::new();
            if let Some(limit) = p.limit {
                parts.push(format!("limit={}", limit));
            }
            if let Some(offset) = p.offset {
                parts.push(format!("offset={}", offset));
            }
            if !parts.is_empty() {
                url = format!("{}?{}", url, parts.join("&"));
            }
        }
        let req = self.http.get(&url);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse(resp).await
    }

    /// Get the total count of records.
    ///
    /// Calls the `@count` endpoint. Returns `ApiError` if the backend
    /// does not support counting.
    pub async fn count(&self) -> Result<usize, ApiError> {
        let url = format!("{}/@count", self.collection_url());
        let req = self.http.get(&url);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        let cr: CountResponse = Self::parse(resp).await?;
        Ok(cr.count)
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

    /// Partially update a record by ID (RFC 7386 JSON Merge Patch).
    ///
    /// Only sends the fields in `patch`. Include `updatedAt` for optimistic
    /// locking — the server returns 409 Conflict if it doesn't match.
    ///
    /// ```ignore
    /// let patch = serde_json::json!({"displayName": "New Name", "updatedAt": "2026-01-01T00:00:00Z"});
    /// let updated = client.patch("abc123", &patch).await?;
    /// ```
    pub async fn patch(&self, id: &str, patch: &serde_json::Value) -> Result<T, ApiError> {
        let req = self.http.patch(&self.item_url(id)).json(patch);
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

// ── Facet list result ───────────────────────────────────────────────

/// List response for facet endpoints. Uses `hasMore` pagination
/// (same shape as admin `ListResponse`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResult<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

// ── FacetClientBase ─────────────────────────────────────────────────

/// Shared HTTP client used by all `#[facet]` generated clients.
///
/// Handles URL construction, authentication, response format negotiation,
/// and response parsing. Generated `{Facet}Client` structs hold a
/// `FacetClientBase` and delegate HTTP operations to it.
///
/// Supports JSON (default) and FlatBuffers wire formats for `list` and
/// `get` operations. Actions always use JSON.
pub struct FacetClientBase {
    http: reqwest::Client,
    base_url: String,
    token_source: Arc<dyn TokenSource>,
    format: Format,
}

impl FacetClientBase {
    pub fn new(base_url: &str, token_source: Arc<dyn TokenSource>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token_source,
            format: Format::Json,
        }
    }

    /// Set the preferred wire format for resource operations (list, get).
    ///
    /// When `FlatBuffers` is selected, requests include
    /// `Accept: application/x-flatbuffers` and responses are decoded
    /// as FlatBuffers. Actions always use JSON regardless of this setting.
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Current wire format.
    pub fn format(&self) -> Format {
        self.format
    }

    async fn authed(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, ApiError> {
        if let Some(token) = self.token_source.token().await? {
            Ok(req.bearer_auth(token))
        } else {
            Ok(req)
        }
    }

    /// Add Accept header based on the configured format.
    fn accept_header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.format {
            Format::FlatBuffers => req.header("Accept", MIME_FLATBUFFERS),
            Format::Json => req,
        }
    }

    /// Detect response format from Content-Type header.
    fn response_format(resp: &reqwest::Response) -> Format {
        resp.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(Format::from_content_type)
            .unwrap_or(Format::Json)
    }

    async fn parse_json<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T, ApiError> {
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::from_response_body(status.as_u16(), &body));
        }
        resp.json::<T>()
            .await
            .map_err(|e| ApiError::Decode(e.to_string()))
    }

    async fn check_status(resp: &reqwest::Response) -> Result<(), ApiError> {
        if !resp.status().is_success() {
            // We can't consume the body here since resp is borrowed,
            // so this is used only as a pre-check pattern.
            return Ok(());
        }
        Ok(())
    }

    /// GET a list endpoint — returns ListResult<T> with hasMore pagination.
    ///
    /// Format-aware: sends Accept header and decodes based on Content-Type.
    pub async fn list<T>(&self, path: &str) -> Result<ListResult<T>, ApiError>
    where
        T: DeserializeOwned + FromFlatBufferList,
    {
        let url = format!("{}{}", self.base_url, path);
        let req = self.accept_header(self.http.get(&url));
        let req = self.authed(req).await?;
        let resp = req.send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::from_response_body(status.as_u16(), &body));
        }

        match Self::response_format(&resp) {
            Format::FlatBuffers => {
                let bytes = resp.bytes().await
                    .map_err(|e| ApiError::Decode(e.to_string()))?;
                let (items, has_more) = T::decode_flatbuffer_list(&bytes)
                    .map_err(|e| ApiError::Decode(e.to_string()))?;
                Ok(ListResult { items, has_more })
            }
            Format::Json => {
                resp.json::<ListResult<T>>()
                    .await
                    .map_err(|e| ApiError::Decode(e.to_string()))
            }
        }
    }

    /// GET a single item endpoint.
    ///
    /// Format-aware: sends Accept header and decodes based on Content-Type.
    pub async fn get<T>(&self, path: &str) -> Result<T, ApiError>
    where
        T: DeserializeOwned + FromFlatBuffer,
    {
        let url = format!("{}{}", self.base_url, path);
        let req = self.accept_header(self.http.get(&url));
        let req = self.authed(req).await?;
        let resp = req.send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::from_response_body(status.as_u16(), &body));
        }

        match Self::response_format(&resp) {
            Format::FlatBuffers => {
                let bytes = resp.bytes().await
                    .map_err(|e| ApiError::Decode(e.to_string()))?;
                T::decode_flatbuffer(&bytes)
                    .map_err(|e| ApiError::Decode(e.to_string()))
            }
            Format::Json => {
                resp.json::<T>()
                    .await
                    .map_err(|e| ApiError::Decode(e.to_string()))
            }
        }
    }

    /// POST with a JSON body (actions — always JSON).
    pub async fn post<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.post(&url).json(body);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse_json(resp).await
    }

    /// POST without a body (actions — always JSON).
    pub async fn post_empty<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<Resp, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.post(&url);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse_json(resp).await
    }

    /// PUT with a JSON body (actions — always JSON).
    pub async fn put<Req: Serialize, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.put(&url).json(body);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse_json(resp).await
    }

    /// PUT without a body (actions — always JSON).
    pub async fn put_empty<Resp: DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<Resp, ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.put(&url);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        Self::parse_json(resp).await
    }

    /// DELETE (no response body, always JSON).
    pub async fn delete(&self, path: &str) -> Result<(), ApiError> {
        let url = format!("{}{}", self.base_url, path);
        let req = self.http.delete(&url);
        let req = self.authed(req).await?;
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::from_response_body(status.as_u16(), &body));
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
    fn from_response_body_parses_structured_error() {
        let body = r#"{"code":"NOT_FOUND","message":"id 'x123' not found"}"#;
        let err = ApiError::from_response_body(404, body);
        assert_eq!(err.status(), Some(404));
        assert_eq!(err.error_code(), Some("NOT_FOUND"));
        assert_eq!(err.message(), "id 'x123' not found");
        assert!(err.is_not_found());
        assert!(!err.is_already_exists());
    }

    #[test]
    fn from_response_body_parses_legacy_error_field() {
        // Legacy format: {"error": "..."} without "code".
        let body = r#"{"error":"something went wrong"}"#;
        let err = ApiError::from_response_body(500, body);
        assert_eq!(err.error_code(), Some("UNKNOWN"));
        assert_eq!(err.message(), "something went wrong");
    }

    #[test]
    fn from_response_body_falls_back_to_raw_text() {
        let err = ApiError::from_response_body(500, "Internal Server Error");
        assert_eq!(err.status(), Some(500));
        assert_eq!(err.error_code(), Some("UNKNOWN"));
        assert_eq!(err.message(), "Internal Server Error");
    }

    #[test]
    fn from_response_body_handles_empty_body() {
        let err = ApiError::from_response_body(502, "");
        assert_eq!(err.status(), Some(502));
        assert_eq!(err.error_code(), Some("UNKNOWN"));
    }

    #[test]
    fn code_based_helpers_match_on_code_not_status() {
        // NOT_FOUND.
        let body = r#"{"code":"NOT_FOUND","message":"x"}"#;
        let err = ApiError::from_response_body(404, body);
        assert!(err.is_not_found());
        assert!(!err.is_already_exists());
        assert!(!err.is_unauthenticated());

        // ALREADY_EXISTS.
        let body = r#"{"code":"ALREADY_EXISTS","message":"dup"}"#;
        let err = ApiError::from_response_body(409, body);
        assert!(err.is_already_exists());
        assert!(!err.is_not_found());

        // UNAUTHENTICATED.
        let body = r#"{"code":"UNAUTHENTICATED","message":"no token"}"#;
        let err = ApiError::from_response_body(401, body);
        assert!(err.is_unauthenticated());
        assert!(!err.is_permission_denied());

        // PERMISSION_DENIED.
        let body = r#"{"code":"PERMISSION_DENIED","message":"no access"}"#;
        let err = ApiError::from_response_body(403, body);
        assert!(err.is_permission_denied());
        assert!(!err.is_unauthenticated());

        // VALIDATION_FAILED.
        let body = r#"{"code":"VALIDATION_FAILED","message":"bad"}"#;
        let err = ApiError::from_response_body(400, body);
        assert!(err.is_validation_failed());

        // READ_ONLY.
        let body = r#"{"code":"READ_ONLY","message":"ro"}"#;
        let err = ApiError::from_response_body(403, body);
        assert!(err.is_read_only());
        // Both READ_ONLY and PERMISSION_DENIED are 403, but code distinguishes.
        assert!(!err.is_permission_denied());
    }

    #[test]
    fn auth_error_helpers() {
        let err = ApiError::Auth("login failed".into());
        assert!(err.is_auth_error());
        assert!(!err.is_network_error());
        assert_eq!(err.status(), None);
        assert_eq!(err.error_code(), None);
        assert_eq!(err.message(), "login failed");
    }

    #[test]
    fn display_format_includes_code() {
        let body = r#"{"code":"NOT_FOUND","message":"id 'abc' not found"}"#;
        let err = ApiError::from_response_body(404, body);
        let display = format!("{}", err);
        assert_eq!(display, "HTTP 404 [NOT_FOUND]: id 'abc' not found");

        let err = ApiError::Auth("login failed (401): invalid credentials".into());
        let display = format!("{}", err);
        assert_eq!(display, "auth: login failed (401): invalid credentials");
    }

    // ── Conflict (optimistic locking) ────────────────────────────────

    #[test]
    fn conflict_error_code_helper() {
        let body = r#"{"code":"CONFLICT","message":"updatedAt mismatch: stored 2026-01-01T00:00:00Z, got 2025-12-31T00:00:00Z"}"#;
        let err = ApiError::from_response_body(409, body);
        assert!(err.is_conflict());
        assert!(!err.is_not_found());
        assert_eq!(err.status(), Some(409));
        assert_eq!(err.error_code(), Some("CONFLICT"));
    }

    // ── ListResponse deserialization ─────────────────────────────────

    #[test]
    fn list_response_deserializes_has_more() {
        let json = r#"{"items":[{"name":"a"},{"name":"b"}],"hasMore":true}"#;
        let resp: ListResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 2);
        assert!(resp.has_more);
    }

    #[test]
    fn list_response_deserializes_no_more() {
        let json = r#"{"items":[],"hasMore":false}"#;
        let resp: ListResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 0);
        assert!(!resp.has_more);
    }

    // ── CountResponse deserialization ────────────────────────────────

    #[test]
    fn count_response_deserializes() {
        let json = r#"{"count":42}"#;
        let resp: CountResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 42);
    }

    // ── ListParams URL building ─────────────────────────────────────

    #[test]
    fn list_params_default_is_empty() {
        let p = ListParams::default();
        assert!(p.limit.is_none());
        assert!(p.offset.is_none());
    }

    #[test]
    fn list_params_builds_query_string() {
        let p = ListParams { limit: Some(20), offset: Some(40) };
        let mut parts = Vec::new();
        if let Some(limit) = p.limit { parts.push(format!("limit={}", limit)); }
        if let Some(offset) = p.offset { parts.push(format!("offset={}", offset)); }
        assert_eq!(parts.join("&"), "limit=20&offset=40");
    }

    // ── Facet ListResult ────────────────────────────────────────────

    #[test]
    fn facet_list_result_deserializes_has_more() {
        let json = r#"{"items":[{"name":"a"},{"name":"b"}],"hasMore":true}"#;
        let result: ListResult<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);
    }

    #[test]
    fn facet_list_result_deserializes_no_more() {
        let json = r#"{"items":[],"hasMore":false}"#;
        let result: ListResult<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(result.items.len(), 0);
        assert!(!result.has_more);
    }
}
