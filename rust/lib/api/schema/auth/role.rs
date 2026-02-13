//! Role resource — named permission set registered by services.
//!
//! Pure db_resource: standard CRUD, no custom endpoints.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "auth", table = "roles", display_name = "Role")]
// #[permission(create = "auth:role:create")]
// #[permission(read = "auth:role:read")]
// #[permission(update = "auth:role:update")]
// #[permission(delete = "auth:role:delete")]
// #[permission(list = "auth:role:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique identifier (e.g. "pms:admin").
    // #[primary_key]
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Permission strings this role grants.
    pub permissions: Vec<String>,

    /// Which service registered this role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,

    // #[auto_timestamp(on_create)]
    pub created_at: String,

    // #[auto_timestamp(on_update)]
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRole {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub permissions: Vec<String>,
    #[serde(default)]
    pub service: Option<String>,
}
