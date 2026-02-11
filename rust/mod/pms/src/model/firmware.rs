use serde::{Deserialize, Serialize};

/// Firmware status.
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

/// A downloadable file within a firmware release.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareFile {
    pub name: String,
    pub url: String,
    pub md5: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// Firmware — device firmware version.
/// Composite PK: model + semver.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Firmware {
    /// Internal ID (model/semver composite key serialized).
    #[serde(default)]
    pub id: String,

    /// Target model code (Model.code).
    pub model: u32,

    /// Semantic version string (e.g. "1.2.3").
    pub semver: String,

    /// Build number (monotonically increasing).
    pub build: u64,

    /// Firmware lifecycle status.
    #[serde(default)]
    pub status: FirmwareStatus,

    /// Downloadable files in this firmware release.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<FirmwareFile>,

    /// Release notes (markdown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,

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

impl Firmware {
    /// Composite key for firmware: "{model}/{semver}".
    pub fn composite_key(model: u32, semver: &str) -> String {
        format!("{}/{}", model, semver)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firmware_json_roundtrip() {
        let fw = Firmware {
            id: "106/1.2.3".into(),
            model: 106,
            semver: "1.2.3".into(),
            build: 42,
            status: FirmwareStatus::Published,
            files: vec![FirmwareFile {
                name: "firmware.bin".into(),
                url: "https://example.com/fw.bin".into(),
                md5: "d41d8cd98f00b204e9800998ecf8427e".into(),
                size: Some(1048576),
            }],
            release_notes: Some("Bug fixes".into()),
            display_name: None,
            description: None,
            data: None,
            create_at: None,
            update_at: None,
        };
        let json = serde_json::to_string(&fw).unwrap();
        let back: Firmware = serde_json::from_str(&json).unwrap();
        assert_eq!(fw, back);
    }

    #[test]
    fn composite_key() {
        assert_eq!(Firmware::composite_key(106, "1.2.3"), "106/1.2.3");
    }
}
