//! SN (Serial Number) encoding and decoding engine.
//!
//! A serial number is 80 bits packed into 16 Crockford Base32 characters,
//! displayed with configurable separator positions.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use pms::sn::{default_config, encode_sn, decode_sn, EncodeInput};
//! use std::collections::HashMap;
//!
//! let config = default_config();
//! let input = EncodeInput {
//!     model_no: 106,
//!     dimensions: HashMap::from([
//!         ("manufacturer".into(), 1u32),
//!         ("channel".into(), 2),
//!     ]),
//!     timestamp: Some((2025, 6)),
//! };
//!
//! let encoded = encode_sn(&config, &input).unwrap();
//! println!("SN: {}", encoded.formatted); // e.g. "1Q8-28394-JMR-PQST-VWXY"
//!
//! let decoded = decode_sn(&config, &encoded.formatted).unwrap();
//! assert_eq!(decoded.model_no(), Some(106));
//! assert_eq!(decoded.year(), Some(2025));
//! ```

pub mod base32;
pub mod config;
pub mod decoder;
pub mod encoder;
pub mod serial_number;

pub use config::{default_config, ConfigError, SNConfig, SegmentDef, SegmentType, TimeField};
pub use decoder::{decode_sn, DecodeError, DecodeOutput, DecodedSegment};
pub use encoder::{encode_sn, EncodeError, EncodeInput, EncodeOutput};
pub use serial_number::{BatchOptions, SerialNumber};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// End-to-end test: encode → format → decode → verify all fields.
    #[test]
    fn end_to_end_encode_decode() {
        let config = default_config();

        let input = EncodeInput {
            model_no: 106,
            dimensions: HashMap::from([
                ("manufacturer".into(), 1u32),
                ("channel".into(), 2),
            ]),
            timestamp: Some((2025, 6)),
        };

        let encoded = encode_sn(&config, &input).unwrap();

        // Raw is 16 chars
        assert_eq!(encoded.raw.len(), 16);
        // Formatted has separators: 16 + 4 = 20
        assert_eq!(encoded.formatted.len(), 20);
        // 10 bytes
        assert_eq!(encoded.bytes.len(), 10);

        // Decode
        let decoded = decode_sn(&config, &encoded.formatted).unwrap();
        assert_eq!(decoded.model_no(), Some(106u64));
        assert_eq!(decoded.year(), Some(2025u64));
        assert_eq!(decoded.week(), Some(6u64));
        assert_eq!(decoded.dimensions().get("manufacturer"), Some(&1u64));
        assert_eq!(decoded.dimensions().get("channel"), Some(&2u64));
        assert_eq!(decoded.values.get("version"), Some(&1u64));
    }

    /// Same input produces different SNs due to random bits.
    #[test]
    fn uniqueness_from_random_bits() {
        let config = default_config();
        let input = EncodeInput {
            model_no: 200,
            dimensions: HashMap::from([
                ("manufacturer".into(), 3u32),
                ("channel".into(), 5),
            ]),
            timestamp: Some((2026, 10)),
        };

        let sn1 = encode_sn(&config, &input).unwrap().formatted;
        let sn2 = encode_sn(&config, &input).unwrap().formatted;
        assert_ne!(sn1, sn2, "two SNs from same input should differ (random bits)");
    }

    /// Config can be serialized to YAML and loaded back.
    #[test]
    fn config_persistence() {
        let config = default_config();
        let yaml = serde_yml::to_string(&config).unwrap();
        let loaded: SNConfig = serde_yml::from_str(&yaml).unwrap();
        loaded.validate().unwrap();
        assert_eq!(config, loaded);
    }

    /// Max values for each segment.
    #[test]
    fn boundary_values() {
        let config = default_config();

        // Max model_no = 255 (8 bits)
        let input = EncodeInput {
            model_no: 255,
            dimensions: HashMap::from([
                ("manufacturer".into(), 63u32), // 6 bits max
                ("channel".into(), 63),          // 6 bits max
            ]),
            timestamp: Some((2020 + 63, 53)), // max year offset 63, max week 53
        };

        let encoded = encode_sn(&config, &input).unwrap();
        let decoded = decode_sn(&config, &encoded.formatted).unwrap();
        assert_eq!(decoded.model_no(), Some(255u64));
        assert_eq!(decoded.year(), Some(2083u64)); // 2020 + 63
        assert_eq!(decoded.week(), Some(53u64));
        assert_eq!(decoded.dimensions().get("manufacturer"), Some(&63u64));
        assert_eq!(decoded.dimensions().get("channel"), Some(&63u64));
    }

    /// Minimum values (all zeros except fixed version).
    #[test]
    fn minimum_values() {
        let config = default_config();

        let input = EncodeInput {
            model_no: 0,
            dimensions: HashMap::from([
                ("manufacturer".into(), 0u32),
                ("channel".into(), 0),
            ]),
            timestamp: Some((2020, 0)),
        };

        let encoded = encode_sn(&config, &input).unwrap();
        let decoded = decode_sn(&config, &encoded.formatted).unwrap();
        assert_eq!(decoded.model_no(), Some(0u64));
        assert_eq!(decoded.year(), Some(2020u64));
        assert_eq!(decoded.week(), Some(0u64));
    }

    /// Base32 character validity — all output chars are in Crockford alphabet.
    #[test]
    fn output_chars_valid() {
        let config = default_config();
        let input = EncodeInput {
            model_no: 42,
            dimensions: HashMap::from([
                ("manufacturer".into(), 7u32),
                ("channel".into(), 3),
            ]),
            timestamp: Some((2025, 15)),
        };

        let encoded = encode_sn(&config, &input).unwrap();
        let crockford = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        for ch in encoded.raw.chars() {
            assert!(
                crockford.contains(ch),
                "char '{}' not in Crockford Base32 alphabet",
                ch
            );
        }
    }
}
