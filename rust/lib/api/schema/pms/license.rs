//! License resource — multi-type license pool.
//!
//! Pure db_resource: standard CRUD.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

// #[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseSource {
    Import,
    Generate,
}

// #[model]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseStatus {
    Available,
    Allocated,
}

impl Default for LicenseStatus {
    fn default() -> Self {
        Self::Available
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "licenses", display_name = "License")]
// #[permission(create = "pms:license:create")]
// #[permission(read = "pms:license:read")]
// #[permission(update = "pms:license:update")]
// #[permission(delete = "pms:license:delete")]
// #[permission(list = "pms:license:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct License {
    // #[primary_key]
    #[serde(default)]
    pub id: String,

    #[serde(rename = "type")]
    pub license_type: String,

    pub number: String,
    pub source: LicenseSource,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sn: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_id: Option<String>,

    #[serde(default)]
    pub status: LicenseStatus,

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

impl License {
    /// Composite key for license: "{type}/{number}".
    pub fn composite_key(license_type: &str, number: &str) -> String {
        format!("{}/{}", license_type, number)
    }
}
