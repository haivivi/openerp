use serde::{Deserialize, Serialize};

/// A role definition containing a set of permission strings.
///
/// Roles are registered by business services. Auth stores and queries them
/// but does not interpret the permission semantics.
///
/// Example:
///   id = "pms:admin"
///   permissions = ["pms:device:read", "pms:device:write", "pms:batch:create"]
///   service = "pms"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique identifier (e.g. "pms:admin", "release:viewer").
    pub id: String,

    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Permission strings this role grants.
    pub permissions: Vec<String>,

    /// Which service registered this role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,

    /// RFC 3339 creation timestamp.
    pub created_at: String,

    /// RFC 3339 last update timestamp.
    pub updated_at: String,
}

/// Input for creating a new role.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRole {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub service: Option<String>,
}
