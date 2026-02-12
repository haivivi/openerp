use serde::{Deserialize, Serialize};

/// SNSegment — a dynamic dimension entry used in SN encoding.
///
/// Each segment belongs to a named dimension (e.g. "manufacturer", "channel")
/// and has a numeric code that gets packed into the SN bit stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SNSegment {
    /// Dimension name (e.g. "manufacturer", "channel").
    pub dimension: String,

    /// Numeric code for this entry (packed into SN bits).
    pub code: u32,

    /// Human-readable name (e.g. "Foxconn", "Tmall").
    pub name: String,

    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_json_roundtrip() {
        let seg = SNSegment {
            dimension: "manufacturer".into(),
            code: 1,
            name: "Foxconn".into(),
            description: Some("Primary manufacturer".into()),
        };
        let json = serde_json::to_string(&seg).unwrap();
        let back: SNSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(seg, back);
    }
}
