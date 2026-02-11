use serde::{Deserialize, Serialize};

/// Model — device model/series definition.
/// Primary key is `code` (the model number used in SN encoding).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// Model code — primary key. Used as the "model" segment in SN encoding.
    pub code: u32,

    /// Series name (e.g. "H106", "H2xx").
    pub series_name: String,

    /// Human-readable display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Extra data (JSON string).
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
    fn model_json_roundtrip() {
        let m = Model {
            code: 106,
            series_name: "H106".into(),
            display_name: Some("H106 Speaker".into()),
            description: None,
            data: None,
            create_at: None,
            update_at: None,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Model = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
