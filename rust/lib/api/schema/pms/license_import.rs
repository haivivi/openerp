//! LicenseImport resource — license import/generation batch record.
//!
//! db_resource + custom API: import (bulk import licenses).

use serde::{Deserialize, Serialize};

use super::license::LicenseSource;

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "license_imports", display_name = "License Import")]
// #[permission(create = "pms:license_import:create")]
// #[permission(read = "pms:license_import:read")]
// #[permission(list = "pms:license_import:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LicenseImport {
    // #[primary_key]
    #[serde(default)]
    pub id: String,

    #[serde(rename = "type")]
    pub license_type: String,

    pub source: LicenseSource,
    pub name: String,

    #[serde(default)]
    pub count: u64,

    #[serde(default)]
    pub allocated_count: u64,

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
pub struct ImportLicensesRequest {
    /// License type.
    #[serde(rename = "type")]
    pub license_type: String,
    /// License numbers to import.
    pub numbers: Vec<String>,
    /// Import name for tracking.
    pub name: String,
}

// ---------------------------------------------------------------------------
// Custom endpoints
// ---------------------------------------------------------------------------
//
// #[api(LicenseImport)]
// #[handlers_path = "crate::handlers::license_import"]
// impl LicenseImportApi {
//     #[endpoint(POST "/pms/license-imports/:id/@import")]
//     #[permission("pms:license_import:import")]
//     #[handler = "import"]
//     async fn import_licenses(id: String, body: ImportLicensesRequest) -> LicenseImport;
// }
