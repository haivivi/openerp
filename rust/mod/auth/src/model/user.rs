use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A user identity. Can be linked to multiple login methods (OAuth providers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier (UUIDv4, no dashes).
    pub id: String,

    /// Display name.
    pub name: String,

    /// Email address (optional, may come from provider).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Avatar URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    /// Whether the user account is active.
    #[serde(default = "default_true")]
    pub active: bool,

    /// Linked accounts: provider_id -> external user id.
    /// e.g. {"github": "12345", "feishu": "ou_abc"}
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub linked_accounts: HashMap<String, String>,

    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// RFC 3339 creation timestamp.
    pub created_at: String,

    /// RFC 3339 last update timestamp.
    pub updated_at: String,
}

/// Input for creating a new user.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateUser {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub linked_accounts: HashMap<String, String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}
