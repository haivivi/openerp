use serde::{Deserialize, Serialize};

/// Batch status.
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

/// Batch — a production batch. @provision generates Devices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    /// UUID primary key.
    #[serde(default)]
    pub id: String,

    /// Batch name (e.g. "H106-2025W06").
    pub name: String,

    /// Target model code (Model.code).
    pub model: u32,

    /// Number of devices to produce.
    pub quantity: u32,

    /// Number of devices already provisioned.
    #[serde(default)]
    pub provisioned_count: u32,

    /// Batch lifecycle status.
    #[serde(default)]
    pub status: BatchStatus,

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
    fn batch_json_roundtrip() {
        let b = Batch {
            id: "batch001".into(),
            name: "H106-2025W06".into(),
            model: 106,
            quantity: 5000,
            provisioned_count: 0,
            status: BatchStatus::Draft,
            display_name: None,
            description: None,
            data: None,
            create_at: None,
            update_at: None,
        };
        let json = serde_json::to_string(&b).unwrap();
        let back: Batch = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }
}
