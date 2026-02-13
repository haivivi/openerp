//! Session resource — JWT session tracking for token refresh and revocation.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "auth", table = "sessions", display_name = "Session")]
// #[permission(create = "auth:session:create")]
// #[permission(read = "auth:session:read")]
// #[permission(delete = "auth:session:delete")]
// #[permission(list = "auth:session:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    // #[primary_key]
    pub id: String,

    // #[index]
    pub user_id: String,

    /// RFC 3339 timestamp when the token was issued.
    pub issued_at: String,

    /// RFC 3339 timestamp when the token expires.
    pub expires_at: String,

    /// Whether this session has been revoked.
    #[serde(default)]
    pub revoked: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
}

// ---------------------------------------------------------------------------
// JWT Claims (model, not a db_resource)
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user id.
    pub sub: String,
    pub name: String,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    /// Session id (for refresh/revoke).
    pub sid: String,
    pub iat: i64,
    pub exp: i64,
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

// #[model]
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Session)]
// #[handlers_path = "crate::handlers::session"]
// impl SessionApi {
//     #[endpoint(POST "/auth/sessions/:id/@revoke")]
//     #[permission("auth:session:revoke")]
//     #[handler = "revoke"]
//     async fn revoke(id: String) -> Session;
//
//     #[endpoint(POST "/auth/token/refresh")]
//     #[public]
//     #[handler = "refresh"]
//     async fn refresh(body: RefreshRequest) -> TokenPair;
// }
