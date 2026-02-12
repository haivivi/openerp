//! SN encoding configuration — defines the bit layout of a serial number.
//!
//! An SN is 80 bits (10 bytes, 16 Crockford Base32 characters).
//! The config specifies an ordered list of segments that pack into those 80 bits.

use serde::{Deserialize, Serialize};

/// The top-level SN configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SNConfig {
    /// Total bits in the SN. Must equal sum of all segment bits. Typically 80.
    pub total_bits: u32,

    /// Separator character inserted into the formatted SN string.
    #[serde(default = "default_separator")]
    pub separator: String,

    /// Character positions (0-indexed into the Base32 string) where separators
    /// are inserted. E.g. `[3, 8, 11, 15]` for "XXX-XXXXX-XXX-XXXX-X".
    #[serde(default)]
    pub separator_positions: Vec<usize>,

    /// Ordered list of segments that pack into the bit stream.
    pub segments: Vec<SegmentDef>,
}

fn default_separator() -> String {
    "-".into()
}

/// Definition of a single segment within the SN bit layout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDef {
    /// Human-readable segment name (e.g. "version", "model", "manufacturer").
    pub name: String,

    /// Segment type — determines how the value is sourced.
    #[serde(rename = "type")]
    pub seg_type: SegmentType,

    /// Number of bits this segment occupies.
    pub bits: u32,

    /// For `fixed` type: the constant value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u32>,

    /// For `dimension` type: the dimension name to look up in SNSegment table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,

    /// For `auto_time` type: which time component to extract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_field: Option<TimeField>,
}

/// Segment type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentType {
    Fixed,
    Model,
    Dimension,
    AutoTime,
    Random,
}

/// Time field variants for `auto_time` segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeField {
    Year,
    Week,
}

impl SNConfig {
    /// Validate the config: total_bits must match sum of segment bits,
    /// and each segment must have the required fields for its type.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let sum: u32 = self.segments.iter().map(|s| s.bits).sum();
        if sum != self.total_bits {
            return Err(ConfigError::BitsMismatch {
                declared: self.total_bits,
                actual: sum,
            });
        }

        if self.total_bits % 8 != 0 {
            return Err(ConfigError::BitsNotByteAligned(self.total_bits));
        }

        for seg in &self.segments {
            seg.validate()?;
        }

        Ok(())
    }

    /// Returns the byte length of the encoded SN (total_bits / 8).
    pub fn byte_len(&self) -> usize {
        (self.total_bits / 8) as usize
    }

    /// Format a raw Base32 string by inserting separators at configured positions.
    pub fn format_sn(&self, raw: &str) -> String {
        if self.separator_positions.is_empty() {
            return raw.to_string();
        }

        let mut result = String::with_capacity(raw.len() + self.separator_positions.len());
        for (i, ch) in raw.chars().enumerate() {
            if self.separator_positions.contains(&i) {
                result.push_str(&self.separator);
            }
            result.push(ch);
        }
        result
    }

    /// Strip separators from a formatted SN string, returning raw Base32.
    pub fn strip_sn(&self, formatted: &str) -> String {
        if self.separator.is_empty() {
            return formatted.to_string();
        }
        formatted.replace(&self.separator, "")
    }
}

impl SegmentDef {
    fn validate(&self) -> Result<(), ConfigError> {
        match self.seg_type {
            SegmentType::Fixed => {
                if self.value.is_none() {
                    return Err(ConfigError::MissingField {
                        segment: self.name.clone(),
                        field: "value".into(),
                    });
                }
                let max = max_value(self.bits);
                if (self.value.unwrap() as u64) > max {
                    return Err(ConfigError::ValueOverflow {
                        segment: self.name.clone(),
                        value: self.value.unwrap() as u64,
                        max,
                    });
                }
            }
            SegmentType::Dimension => {
                if self.dimension.is_none() {
                    return Err(ConfigError::MissingField {
                        segment: self.name.clone(),
                        field: "dimension".into(),
                    });
                }
            }
            SegmentType::AutoTime => {
                if self.time_field.is_none() {
                    return Err(ConfigError::MissingField {
                        segment: self.name.clone(),
                        field: "timeField".into(),
                    });
                }
            }
            SegmentType::Model | SegmentType::Random => {}
        }
        Ok(())
    }
}

/// Maximum unsigned value that fits in `bits` bits.
pub(crate) fn max_value(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// SN config validation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("total_bits ({declared}) does not match sum of segment bits ({actual})")]
    BitsMismatch { declared: u32, actual: u32 },

    #[error("total_bits ({0}) is not byte-aligned (must be multiple of 8)")]
    BitsNotByteAligned(u32),

    #[error("segment '{segment}' missing required field '{field}'")]
    MissingField { segment: String, field: String },

    #[error("segment '{segment}' value {value} overflows {max} (max for bit width)")]
    ValueOverflow { segment: String, value: u64, max: u64 },
}

/// Construct the default SNConfig used by Haivivi PMS.
///
/// Layout (80 bits total):
/// - version: 4 bits (fixed = 1)
/// - model: 8 bits (DeviceSeries.seriesNo)
/// - manufacturer: 6 bits (dimension)
/// - channel: 6 bits (dimension)
/// - year: 6 bits (auto_time, offset from 2020)
/// - week: 6 bits (auto_time, ISO week 1-53)
/// - random: 44 bits
pub fn default_config() -> SNConfig {
    SNConfig {
        total_bits: 80,
        separator: "-".into(),
        separator_positions: vec![3, 8, 11, 15],
        segments: vec![
            SegmentDef {
                name: "version".into(),
                seg_type: SegmentType::Fixed,
                bits: 4,
                value: Some(1),
                dimension: None,
                time_field: None,
            },
            SegmentDef {
                name: "model".into(),
                seg_type: SegmentType::Model,
                bits: 8,
                value: None,
                dimension: None,
                time_field: None,
            },
            SegmentDef {
                name: "manufacturer".into(),
                seg_type: SegmentType::Dimension,
                bits: 6,
                value: None,
                dimension: Some("manufacturer".into()),
                time_field: None,
            },
            SegmentDef {
                name: "channel".into(),
                seg_type: SegmentType::Dimension,
                bits: 6,
                value: None,
                dimension: Some("channel".into()),
                time_field: None,
            },
            SegmentDef {
                name: "year".into(),
                seg_type: SegmentType::AutoTime,
                bits: 6,
                value: None,
                dimension: None,
                time_field: Some(TimeField::Year),
            },
            SegmentDef {
                name: "week".into(),
                seg_type: SegmentType::AutoTime,
                bits: 6,
                value: None,
                dimension: None,
                time_field: Some(TimeField::Week),
            },
            SegmentDef {
                name: "random".into(),
                seg_type: SegmentType::Random,
                bits: 44,
                value: None,
                dimension: None,
                time_field: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = default_config();
        cfg.validate().unwrap();
        assert_eq!(cfg.byte_len(), 10);
    }

    #[test]
    fn config_yaml_roundtrip() {
        let cfg = default_config();
        let yaml = serde_yml::to_string(&cfg).unwrap();
        let back: SNConfig = serde_yml::from_str(&yaml).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn config_json_roundtrip() {
        let cfg = default_config();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let back: SNConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn validate_bits_mismatch() {
        let mut cfg = default_config();
        cfg.total_bits = 64;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, ConfigError::BitsMismatch { .. }));
    }

    #[test]
    fn validate_missing_fixed_value() {
        let mut cfg = default_config();
        cfg.segments[0].value = None;
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, ConfigError::MissingField { .. }));
    }

    #[test]
    fn validate_fixed_value_overflow() {
        let mut cfg = default_config();
        cfg.segments[0].value = Some(16); // 4 bits max = 15
        let err = cfg.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValueOverflow { .. }));
    }

    #[test]
    fn format_and_strip_sn() {
        let cfg = default_config();
        let raw = "0123456789ABCDEF";
        let formatted = cfg.format_sn(raw);
        assert_eq!(formatted, "012-34567-89A-BCDE-F");
        let stripped = cfg.strip_sn(&formatted);
        assert_eq!(stripped, raw);
    }

    #[test]
    fn format_no_separators() {
        let cfg = SNConfig {
            total_bits: 80,
            separator: "-".into(),
            separator_positions: vec![],
            segments: vec![],
        };
        let raw = "0123456789ABCDEF";
        assert_eq!(cfg.format_sn(raw), raw);
    }
}
