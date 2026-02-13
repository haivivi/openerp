//! TaskType resource — runtime-registered task type definition.
//!
//! Pure db_resource: standard CRUD.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "task", table = "task_types", display_name = "Task Type")]
// #[permission(create = "task:task_type:create")]
// #[permission(read = "task:task_type:read")]
// #[permission(update = "task:task_type:update")]
// #[permission(delete = "task:task_type:delete")]
// #[permission(list = "task:task_type:list")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskType {
    /// Unique type key, e.g. "pms.batch.provision".
    // #[primary_key]
    #[serde(rename = "type")]
    pub task_type: String,

    /// Owning service name, e.g. "pms".
    pub service: String,

    #[serde(default)]
    pub description: String,

    /// Default timeout in seconds (0 = no limit).
    #[serde(default)]
    pub default_timeout: i64,

    /// Max concurrent running tasks of this type (0 = unlimited).
    #[serde(default)]
    pub max_concurrency: i64,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterTaskTypeRequest {
    #[serde(rename = "type")]
    pub task_type: String,
    pub service: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_timeout: i64,
    #[serde(default)]
    pub max_concurrency: i64,
}
