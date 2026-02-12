use serde::{Deserialize, Serialize};

use super::LicenseSource;

/// LicenseImport — a record of one license import or generation batch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LicenseImport {
    /// UUID primary key.
    #[serde(default)]
    pub id: String,

    /// License type (e.g. "MIIT").
    #[serde(rename = "type")]
    pub license_type: String,

    /// Source: import or generate.
    pub source: LicenseSource,

    /// Descriptive name (e.g. "2025-Q1 MIIT batch").
    pub name: String,

    /// Total count of licenses in this import/generation.
    #[serde(default)]
    pub count: u64,

    /// Number of licenses already allocated to devices.
    #[serde(default)]
    pub allocated_count: u64,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_at: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn license_import_json_roundtrip() {
        let li = LicenseImport {
            id: "import001".into(),
            license_type: "MIIT".into(),
            source: LicenseSource::Import,
            name: "2025-Q1 MIIT batch".into(),
            count: 10000,
            allocated_count: 3500,
            display_name: None,
            description: None,
            data: None,
            create_at: None,
            update_at: None,
        };
        let json = serde_json::to_string(&li).unwrap();
        let back: LicenseImport = serde_json::from_str(&json).unwrap();
        assert_eq!(li, back);
    }
}
