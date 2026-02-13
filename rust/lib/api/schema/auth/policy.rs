//! Policy resource — ACL entry (who, what, how, expires_at).
//!
//! db_resource + custom check endpoint.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "auth", table = "policies", display_name = "Policy")]
// #[permission(create = "auth:policy:create")]
// #[permission(read = "auth:policy:read")]
// #[permission(update = "auth:policy:update")]
// #[permission(delete = "auth:policy:delete")]
// #[permission(list = "auth:policy:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Deterministic id: hex(sha256(who + ":" + what + ":" + how)).
    // #[primary_key]
    pub id: String,

    /// Subject: "user:{id}" or "group:{id}".
    // #[index]
    pub who: String,

    /// Resource path (empty string = global).
    #[serde(default)]
    pub what: String,

    /// Role id that this policy grants.
    pub how: String,

    /// Optional expiration (RFC 3339). None = permanent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,

    // #[auto_timestamp(on_create)]
    pub created_at: String,

    // #[auto_timestamp(on_update)]
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePolicy {
    pub who: String,
    #[serde(default)]
    pub what: String,
    pub how: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyQuery {
    #[serde(default)]
    pub who: Option<String>,
    #[serde(default)]
    pub what: Option<String>,
    #[serde(default)]
    pub how: Option<String>,
}

// ---------------------------------------------------------------------------
// Check types
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckParams {
    pub who: String,
    #[serde(default)]
    pub what: String,
    pub how: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute the deterministic policy id from (who, what, how).
pub fn policy_id(who: &str, what: &str, how: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(who.as_bytes());
    hasher.update(b":");
    hasher.update(what.as_bytes());
    hasher.update(b":");
    hasher.update(how.as_bytes());
    let result = hasher.finalize();
    result[..16]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Policy)]
// #[handlers_path = "crate::handlers::policy"]
// impl PolicyApi {
//     #[endpoint(POST "/auth/check")]
//     #[permission("auth:policy:check")]
//     #[handler = "check"]
//     async fn check(body: CheckParams) -> CheckResult;
// }
