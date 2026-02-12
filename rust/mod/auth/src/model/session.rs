use serde::{Deserialize, Serialize};

/// A JWT session record, used for token refresh and revocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session id (UUIDv4, no dashes).
    pub id: String,

    /// User id that owns this session.
    pub user_id: String,

    /// RFC 3339 timestamp when the token was issued.
    pub issued_at: String,

    /// RFC 3339 timestamp when the token expires.
    pub expires_at: String,

    /// Whether this session has been revoked.
    #[serde(default)]
    pub revoked: bool,

    /// User agent or device info (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// IP address at creation (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
}

/// JWT claims payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: user id.
    pub sub: String,

    /// User display name.
    pub name: String,

    /// Groups the user belongs to (expanded).
    #[serde(default)]
    pub groups: Vec<String>,

    /// Roles assigned to the user (via policies).
    #[serde(default)]
    pub roles: Vec<String>,

    /// Session id (for refresh/revoke).
    pub sid: String,

    /// Issued at (unix timestamp).
    pub iat: i64,

    /// Expiration (unix timestamp).
    pub exp: i64,
}

/// Request body for token refresh.
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Token pair returned after login or refresh.
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}
