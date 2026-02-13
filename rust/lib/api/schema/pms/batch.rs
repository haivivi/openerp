//! Batch resource — a production batch. @provision generates Devices.
//!
//! db_resource + custom API: provision.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BatchStatus {
    Draft,
    Provisioning,
    Completed,
    Cancelled,
}

impl Default for BatchStatus {
    fn default() -> Self {
        Self::Draft
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "batches", display_name = "Batch")]
// #[permission(create = "pms:batch:create")]
// #[permission(read = "pms:batch:read")]
// #[permission(update = "pms:batch:update")]
// #[permission(delete = "pms:batch:delete")]
// #[permission(list = "pms:batch:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    // #[primary_key]
    #[serde(default)]
    pub id: String,

    pub name: String,
    pub model: u32,
    pub quantity: u32,

    #[serde(default)]
    pub provisioned_count: u32,

    #[serde(default)]
    pub status: BatchStatus,

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

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionBatchRequest {
    /// Number of devices to provision in this request (may be < quantity for partial).
    #[serde(default)]
    pub count: Option<u32>,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Batch)]
// #[handlers_path = "crate::handlers::batch"]
// impl BatchApi {
//     #[endpoint(POST "/pms/batches/:id/@provision")]
//     #[permission("pms:batch:provision")]
//     #[handler = "provision"]
//     async fn provision(id: String, body: ProvisionBatchRequest) -> Batch;
// }
