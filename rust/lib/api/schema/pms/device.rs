//! Device resource — a single produced device, created by Batch provisioning.
//!
//! db_resource + custom APIs: provision, activate.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Models (enums)
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceStatus {
    Pending,
    Provisioned,
    Activated,
    Retired,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self::Pending
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "devices", display_name = "Device")]
// #[permission(create = "pms:device:create")]
// #[permission(read = "pms:device:read")]
// #[permission(update = "pms:device:update")]
// #[permission(delete = "pms:device:delete")]
// #[permission(list = "pms:device:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// Device serial number — primary key.
    // #[primary_key]
    pub sn: String,

    /// Device secret (unique).
    pub secret: String,

    /// Target model code.
    pub model: u32,

    // #[default(DeviceStatus::Pending)]
    #[serde(default)]
    pub status: DeviceStatus,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imei: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<String>,

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
pub struct ProvisionRequest {
    /// IMEI numbers to assign.
    #[serde(default)]
    pub imei_list: Vec<String>,
    /// License IDs to assign.
    #[serde(default)]
    pub license_ids: Vec<String>,
    /// Additional data.
    #[serde(default)]
    pub data: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateRequest {
    /// Optional activation data.
    #[serde(default)]
    pub data: Option<String>,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Device)]
// #[handlers_path = "crate::handlers::device"]
// impl DeviceApi {
//     #[endpoint(POST "/pms/devices/:sn/@provision")]
//     #[permission("pms:device:provision")]
//     #[handler = "provision"]
//     async fn provision(sn: String, body: ProvisionRequest) -> Device;
//
//     #[endpoint(POST "/pms/devices/:sn/@activate")]
//     #[permission("pms:device:activate")]
//     #[handler = "activate"]
//     async fn activate(sn: String, body: ActivateRequest) -> Device;
// }
