//! Firmware resource — device firmware versions.
//!
//! db_resource + custom API: upload.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FirmwareStatus {
    Draft,
    Published,
    Deprecated,
}

impl Default for FirmwareStatus {
    fn default() -> Self {
        Self::Draft
    }
}

// #[model]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFile {
    pub name: String,
    pub url: String,
    pub md5: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "firmwares", display_name = "Firmware")]
// #[permission(create = "pms:firmware:create")]
// #[permission(read = "pms:firmware:read")]
// #[permission(update = "pms:firmware:update")]
// #[permission(delete = "pms:firmware:delete")]
// #[permission(list = "pms:firmware:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Firmware {
    // #[primary_key]
    #[serde(default)]
    pub id: String,

    pub model: u32,
    pub semver: String,
    pub build: u64,

    #[serde(default)]
    pub status: FirmwareStatus,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FirmwareFile>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,

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

impl Firmware {
    /// Composite key: "{model}/{semver}".
    pub fn composite_key(model: u32, semver: &str) -> String {
        format!("{}/{}", model, semver)
    }
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(Firmware)]
// #[handlers_path = "crate::handlers::firmware"]
// impl FirmwareApi {
//     #[endpoint(POST "/pms/firmwares/:id/@upload")]
//     #[permission("pms:firmware:upload")]
//     #[handler = "upload"]
//     async fn upload(id: String, body: Multipart) -> Firmware;
// }
