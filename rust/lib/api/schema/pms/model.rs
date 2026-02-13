//! Model resource — device model/series definition.
//!
//! Pure db_resource: standard CRUD.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "models", display_name = "Model")]
// #[permission(create = "pms:model:create")]
// #[permission(read = "pms:model:read")]
// #[permission(update = "pms:model:update")]
// #[permission(delete = "pms:model:delete")]
// #[permission(list = "pms:model:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// Model code — primary key. Used as the "model" segment in SN encoding.
    // #[primary_key]
    pub code: u32,

    /// Series name (e.g. "H106", "H2xx").
    pub series_name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    // #[auto_timestamp(on_create)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_at: Option<String>,

    // #[auto_timestamp(on_update)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_at: Option<String>,
}
