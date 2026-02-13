//! Group resource — hierarchical organization unit.
//!
//! db_resource + custom APIs for member management.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "auth", table = "groups", display_name = "Group")]
// #[permission(create = "auth:group:create")]
// #[permission(read = "auth:group:read")]
// #[permission(update = "auth:group:update")]
// #[permission(delete = "auth:group:delete")]
// #[permission(list = "auth:group:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    // #[primary_key]
    pub id: String,

    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Parent group id (None = top-level).
    // #[index]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// External source type for member sync (e.g. "feishu", "github").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_source: Option<String>,

    /// External identifier for syncing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    // #[auto_timestamp(on_create)]
    pub created_at: String,

    // #[auto_timestamp(on_update)]
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Member management types
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub group_id: String,
    /// Member reference: "user:{user_id}" or "group:{group_id}".
    pub member_ref: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddGroupMember {
    pub member_ref: String,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Group)]
// #[handlers_path = "crate::handlers::group"]
// impl GroupApi {
//     #[endpoint(POST "/auth/groups/:id/@members")]
//     #[permission("auth:group:add_member")]
//     #[handler = "add_member"]
//     async fn add_member(id: String, body: AddGroupMember) -> GroupMember;
//
//     #[endpoint(DELETE "/auth/groups/:id/@members/:member_ref")]
//     #[permission("auth:group:remove_member")]
//     #[handler = "remove_member"]
//     async fn remove_member(id: String, member_ref: String) -> ();
//
//     #[endpoint(GET "/auth/groups/:id/@members")]
//     #[permission("auth:group:read")]
//     #[handler = "list_members"]
//     async fn list_members(id: String) -> Vec<GroupMember>;
// }
