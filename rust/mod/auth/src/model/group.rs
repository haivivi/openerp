use serde::{Deserialize, Serialize};

/// A hierarchical group / organization unit.
///
/// Groups form a tree via `parent_id`. Members can be direct users,
/// child groups, or external references (e.g. Feishu department, GitHub team)
/// that are periodically synced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    /// Unique identifier (UUIDv4, no dashes).
    pub id: String,

    /// Group display name.
    pub name: String,

    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Parent group id (None = top-level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// External source type for member sync (e.g. "feishu", "github").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_source: Option<String>,

    /// External identifier for syncing (e.g. department_id, team slug).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    /// RFC 3339 creation timestamp.
    pub created_at: String,

    /// RFC 3339 last update timestamp.
    pub updated_at: String,
}

/// Input for creating a new group.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroup {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub external_source: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
}

/// A membership record linking a user to a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    /// Group id.
    pub group_id: String,

    /// Member reference: "user:{user_id}" or "group:{group_id}".
    pub member_ref: String,

    /// RFC 3339 timestamp when the membership was added.
    pub added_at: String,
}

/// Input for adding a member to a group.
#[derive(Debug, Clone, Deserialize)]
pub struct AddGroupMember {
    /// Member reference: "user:{user_id}" or "group:{group_id}".
    pub member_ref: String,
}
