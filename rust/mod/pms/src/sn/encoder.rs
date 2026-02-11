//! SN encoder — packs segment values into a bit stream and encodes to Base32.

use std::collections::HashMap;

use rand::Rng;

use super::base32;
use super::config::{max_value, SNConfig, SegmentType, TimeField};

/// Base year for the `year` time field. Year values are stored as offset from this.
const BASE_YEAR: u32 = 2020;

/// Input parameters for SN encoding.
#[derive(Debug, Clone)]
pub struct EncodeInput {
    /// Device series number (Model.code).
    pub model_no: u32,

    /// Dimension values: dimension_name → code.
    pub dimensions: HashMap<String, u32>,

    /// Production timestamp as (year, iso_week).
    /// If None, uses current time.
    pub timestamp: Option<(u32, u32)>,
}

/// Result of SN encoding.
#[derive(Debug, Clone)]
pub struct EncodeOutput {
    /// Raw Base32 string (no separators), 16 characters for 80-bit SN.
    pub raw: String,

    /// Formatted SN with separators.
    pub formatted: String,

    /// The underlying bytes (10 bytes for 80-bit SN).
    pub bytes: Vec<u8>,
}

/// Errors during SN encoding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("config validation failed: {0}")]
    ConfigInvalid(#[from] super::config::ConfigError),

    #[error("missing dimension '{0}' in input")]
    MissingDimension(String),

    #[error("segment '{segment}' value {value} overflows max {max} for {bits} bits")]
    ValueOverflow {
        segment: String,
        value: u64,
        max: u64,
        bits: u32,
    },
}

/// Encode an SN from the given config and input parameters.
pub fn encode_sn(config: &SNConfig, input: &EncodeInput) -> Result<EncodeOutput, EncodeError> {
    config.validate()?;

    let byte_len = config.byte_len();
    let mut buffer = vec![0u8; byte_len];
    let mut bit_offset: u32 = 0;
    let mut rng = rand::thread_rng();

    for seg in &config.segments {
        let value: u64 = match seg.seg_type {
            SegmentType::Fixed => seg.value.unwrap() as u64,

            SegmentType::Model => input.model_no as u64,

            SegmentType::Dimension => {
                let dim = seg.dimension.as_ref().unwrap();
                let code = *input
                    .dimensions
                    .get(dim)
                    .ok_or_else(|| EncodeError::MissingDimension(dim.clone()))?;
                code as u64
            }

            SegmentType::AutoTime => {
                let (year, week) = input.timestamp.unwrap_or_else(current_year_week);
                match seg.time_field.unwrap() {
                    TimeField::Year => year.saturating_sub(BASE_YEAR) as u64,
                    TimeField::Week => week as u64,
                }
            }

            SegmentType::Random => {
                let max = max_value(seg.bits);
                if seg.bits >= 64 {
                    rng.gen::<u64>()
                } else {
                    rng.gen_range(0..=max)
                }
            }
        };

        // Check overflow for non-random segments
        if seg.seg_type != SegmentType::Random {
            let max = max_value(seg.bits);
            if value > max {
                return Err(EncodeError::ValueOverflow {
                    segment: seg.name.clone(),
                    value,
                    max,
                    bits: seg.bits,
                });
            }
        }

        pack_bits(&mut buffer, bit_offset, seg.bits, value);
        bit_offset += seg.bits;
    }

    let raw = base32::encode(&buffer);
    let formatted = config.format_sn(&raw);

    Ok(EncodeOutput {
        raw,
        formatted,
        bytes: buffer,
    })
}

/// Pack `num_bits` of `value` into `buffer` starting at `bit_offset`.
/// Bits are packed MSB-first (big-endian bit order).
fn pack_bits(buffer: &mut [u8], bit_offset: u32, num_bits: u32, value: u64) {
    for i in 0..num_bits {
        let bit = ((value >> (num_bits - 1 - i)) & 1) as u8;
        let pos = bit_offset + i;
        let byte_idx = (pos / 8) as usize;
        let bit_idx = 7 - (pos % 8);
        if bit == 1 {
            buffer[byte_idx] |= 1 << bit_idx;
        }
    }
}

/// Get current year and ISO week number.
fn current_year_week() -> (u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let days = (secs / 86400) as u32;

    let mut year = 1970u32;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let week = (remaining / 7) + 1;
    (year, week.min(53))
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sn::config::default_config;

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
    fn encode_produces_16_char_base32() {
        let cfg = default_config();
        let output = encode_sn(&cfg, &test_input()).unwrap();
        assert_eq!(output.raw.len(), 16);
        assert_eq!(output.bytes.len(), 10);
    }

    #[test]
    fn encode_formatted_has_separators() {
        let cfg = default_config();
        let output = encode_sn(&cfg, &test_input()).unwrap();
        assert_eq!(output.formatted.len(), 20);
        assert_eq!(output.formatted.matches('-').count(), 4);
    }

    #[test]
    fn encode_deterministic_bits_except_random() {
        let cfg = default_config();
        let input = test_input();
        let out1 = encode_sn(&cfg, &input).unwrap();
        let out2 = encode_sn(&cfg, &input).unwrap();

        assert_eq!(out1.bytes[0], out2.bytes[0]);
        assert_eq!(out1.bytes[1], out2.bytes[1]);
        assert_eq!(out1.bytes[2], out2.bytes[2]);
        assert_eq!(out1.bytes[3], out2.bytes[3]);
        assert_eq!(out1.bytes[4] & 0xF0, out2.bytes[4] & 0xF0);
    }

    #[test]
    fn encode_different_random_each_time() {
        let cfg = default_config();
        let input = test_input();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..100 {
            let output = encode_sn(&cfg, &input).unwrap();
            seen.insert(output.raw);
        }
        assert!(seen.len() >= 99);
    }

    #[test]
    fn encode_missing_dimension() {
        let cfg = default_config();
        let input = EncodeInput {
            model_no: 106,
            dimensions: HashMap::new(),
            timestamp: Some((2025, 6)),
        };
        let err = encode_sn(&cfg, &input).unwrap_err();
        assert!(matches!(err, EncodeError::MissingDimension(_)));
    }

    #[test]
    fn encode_model_overflow() {
        let cfg = default_config();
        let mut input = test_input();
        input.model_no = 256;
        let err = encode_sn(&cfg, &input).unwrap_err();
        assert!(matches!(err, EncodeError::ValueOverflow { .. }));
    }

    #[test]
    fn pack_bits_basic() {
        let mut buf = [0u8; 2];
        pack_bits(&mut buf, 0, 5, 0b11010u64);
        assert_eq!(buf[0], 0b11010_000);
        pack_bits(&mut buf, 5, 3, 0b101u64);
        assert_eq!(buf[0], 0b11010_101);
    }

    #[test]
    fn pack_bits_cross_byte() {
        let mut buf = [0u8; 2];
        pack_bits(&mut buf, 4, 8, 0xABu64);
        assert_eq!(buf[0], 0x0A);
        assert_eq!(buf[1], 0xB0);
    }
}
