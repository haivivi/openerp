use serde::{Deserialize, Serialize};

/// Device status.
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

/// Device — a single produced device, created by Batch provisioning.
/// PK = sn (serial number).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// Device serial number — primary key.
    pub sn: String,

    /// Device secret (unique). Used for device authentication.
    pub secret: String,

    /// Target model code (Model.code).
    pub model: u32,

    /// Device status.
    #[serde(default)]
    pub status: DeviceStatus,

    /// SKU identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,

    /// IMEI numbers assigned to this device.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imei: Vec<String>,

    /// License numbers assigned to this device.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<String>,

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
    fn device_json_roundtrip() {
        let d = Device {
            sn: "HVV-A1B2C-3D4-E5F6-7G8H".into(),
            secret: "a1b2c3d4".into(),
            model: 106,
            status: DeviceStatus::Provisioned,
            sku: None,
            imei: vec![],
            licenses: vec![],
            display_name: None,
            description: None,
            data: None,
            create_at: None,
            update_at: None,
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: Device = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
