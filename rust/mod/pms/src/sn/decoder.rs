//! SN decoder — unpacks a Base32-encoded SN back into segment values.

use std::collections::HashMap;

use super::base32;
use super::config::{SNConfig, SegmentType, TimeField};

/// Base year for the `year` time field (must match encoder).
const BASE_YEAR: u32 = 2020;

/// A single decoded segment with its raw value and interpreted meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSegment {
    pub name: String,
    pub seg_type: SegmentType,
    pub raw_value: u64,
    /// For `auto_time` year: the absolute year (raw + BASE_YEAR).
    /// For other types: same as raw_value.
    pub interpreted_value: u64,
    pub dimension: Option<String>,
}

/// Result of decoding an SN.
#[derive(Debug, Clone)]
pub struct DecodeOutput {
    pub segments: Vec<DecodedSegment>,
    pub values: HashMap<String, u64>,
    pub bytes: Vec<u8>,
}

impl DecodeOutput {
    pub fn model_no(&self) -> Option<u64> {
        self.segments
            .iter()
            .find(|s| s.seg_type == SegmentType::Model)
            .map(|s| s.raw_value)
    }

    pub fn year(&self) -> Option<u64> {
        self.segments
            .iter()
            .find(|s| s.seg_type == SegmentType::AutoTime && s.name.contains("year"))
            .map(|s| s.interpreted_value)
    }

    pub fn week(&self) -> Option<u64> {
        self.segments
            .iter()
            .find(|s| s.seg_type == SegmentType::AutoTime && s.name.contains("week"))
            .map(|s| s.interpreted_value)
    }

    pub fn dimensions(&self) -> HashMap<String, u64> {
        self.segments
            .iter()
            .filter(|s| s.seg_type == SegmentType::Dimension)
            .filter_map(|s| s.dimension.as_ref().map(|d| (d.clone(), s.raw_value)))
            .collect()
    }
}

/// Errors during SN decoding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("base32 decode failed: {0}")]
    Base32(#[from] super::base32::Base32Error),

    #[error("config validation failed: {0}")]
    ConfigInvalid(#[from] super::config::ConfigError),

    #[error("decoded {actual} bytes but config expects {expected}")]
    LengthMismatch { expected: usize, actual: usize },

    #[error("fixed segment '{name}' value {actual} does not match expected {expected}")]
    FixedMismatch {
        name: String,
        expected: u64,
        actual: u64,
    },
}

/// Decode a formatted or raw SN string using the given config.
pub fn decode_sn(config: &SNConfig, sn: &str) -> Result<DecodeOutput, DecodeError> {
    config.validate()?;

    let raw = config.strip_sn(sn);
    let bytes = base32::decode(&raw)?;

    let expected_len = config.byte_len();
    if bytes.len() != expected_len {
        return Err(DecodeError::LengthMismatch {
            expected: expected_len,
            actual: bytes.len(),
        });
    }

    let mut segments = Vec::with_capacity(config.segments.len());
    let mut values = HashMap::new();
    let mut bit_offset: u32 = 0;

    for seg in &config.segments {
        let raw_value = extract_bits(&bytes, bit_offset, seg.bits);
        bit_offset += seg.bits;

        if seg.seg_type == SegmentType::Fixed {
            let expected = seg.value.unwrap() as u64;
            if raw_value != expected {
                return Err(DecodeError::FixedMismatch {
                    name: seg.name.clone(),
                    expected,
                    actual: raw_value,
                });
            }
        }

        let interpreted_value = match seg.seg_type {
            SegmentType::AutoTime => match seg.time_field {
                Some(TimeField::Year) => raw_value + BASE_YEAR as u64,
                _ => raw_value,
            },
            _ => raw_value,
        };

        values.insert(seg.name.clone(), raw_value);
        segments.push(DecodedSegment {
            name: seg.name.clone(),
            seg_type: seg.seg_type,
            raw_value,
            interpreted_value,
            dimension: seg.dimension.clone(),
        });
    }

    Ok(DecodeOutput {
        segments,
        values,
        bytes,
    })
}

/// Extract `num_bits` bits from `buffer` starting at `bit_offset`.
fn extract_bits(buffer: &[u8], bit_offset: u32, num_bits: u32) -> u64 {
    let mut value: u64 = 0;
    for i in 0..num_bits {
        let pos = bit_offset + i;
        let byte_idx = (pos / 8) as usize;
        let bit_idx = 7 - (pos % 8);
        let bit = (buffer[byte_idx] >> bit_idx) & 1;
        value = (value << 1) | (bit as u64);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sn::config::default_config;
    use crate::sn::encoder::{encode_sn, EncodeInput};

    fn test_input() -> EncodeInput {
        EncodeInput {
            model_no: 106,
            dimensions: HashMap::from([
                ("manufacturer".into(), 1),
                ("channel".into(), 2),
            ]),
            timestamp: Some((2025, 6)),
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let cfg = default_config();
        let encoded = encode_sn(&cfg, &test_input()).unwrap();
        let decoded = decode_sn(&cfg, &encoded.formatted).unwrap();

        assert_eq!(decoded.model_no(), Some(106u64));
        assert_eq!(decoded.year(), Some(2025u64));
        assert_eq!(decoded.week(), Some(6u64));
        assert_eq!(decoded.dimensions().get("manufacturer"), Some(&1u64));
        assert_eq!(decoded.dimensions().get("channel"), Some(&2u64));
        assert_eq!(decoded.values.get("version"), Some(&1u64));
        assert_eq!(decoded.bytes, encoded.bytes);
    }

    #[test]
    fn decode_raw_string() {
        let cfg = default_config();
        let encoded = encode_sn(&cfg, &test_input()).unwrap();
        let decoded = decode_sn(&cfg, &encoded.raw).unwrap();
        assert_eq!(decoded.model_no(), Some(106u64));
    }

    #[test]
    fn decode_case_insensitive() {
        let cfg = default_config();
        let encoded = encode_sn(&cfg, &test_input()).unwrap();
        let lower = encoded.formatted.to_lowercase();
        let decoded = decode_sn(&cfg, &lower).unwrap();
        assert_eq!(decoded.model_no(), Some(106u64));
    }

    #[test]
    fn decode_wrong_version_fails() {
        let cfg = default_config();
        let encoded = encode_sn(&cfg, &test_input()).unwrap();
        let mut bad_bytes = encoded.bytes.clone();
        bad_bytes[0] ^= 0xF0;
        let bad_raw = crate::sn::base32::encode(&bad_bytes);
        let bad_formatted = cfg.format_sn(&bad_raw);
        let err = decode_sn(&cfg, &bad_formatted).unwrap_err();
        assert!(matches!(err, DecodeError::FixedMismatch { .. }));
    }

    #[test]
    fn decode_wrong_length_fails() {
        let cfg = default_config();
        let err = decode_sn(&cfg, "ABC").unwrap_err();
        assert!(matches!(err, DecodeError::LengthMismatch { .. }));
    }

    #[test]
    fn roundtrip_many_random() {
        let cfg = default_config();
        let input = test_input();
        for _ in 0..100 {
            let encoded = encode_sn(&cfg, &input).unwrap();
            let decoded = decode_sn(&cfg, &encoded.formatted).unwrap();
            assert_eq!(decoded.model_no(), Some(106u64));
            assert_eq!(decoded.year(), Some(2025u64));
            assert_eq!(decoded.week(), Some(6u64));
        }
    }
}
