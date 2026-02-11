//! Crockford Base32 encoding/decoding for SN generation.
//!
//! Crockford Base32 uses the alphabet `0123456789ABCDEFGHJKMNPQRSTVWXYZ`
//! (excludes I, L, O, U to avoid ambiguity with 1, L, 0, V).
//!
//! Reference: <https://www.crockford.com/base32.html>

/// Crockford Base32 encoding alphabet (32 symbols).
const ENCODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Decode table: maps ASCII byte → 5-bit value (255 = invalid).
/// Handles both uppercase and lowercase, plus common substitutions
/// (O/o → 0, I/i/L/l → 1).
const DECODE_TABLE: [u8; 128] = {
    let mut table = [255u8; 128];

    // Digits 0-9
    table[b'0' as usize] = 0;
    table[b'1' as usize] = 1;
    table[b'2' as usize] = 2;
    table[b'3' as usize] = 3;
    table[b'4' as usize] = 4;
    table[b'5' as usize] = 5;
    table[b'6' as usize] = 6;
    table[b'7' as usize] = 7;
    table[b'8' as usize] = 8;
    table[b'9' as usize] = 9;

    // Uppercase letters
    table[b'A' as usize] = 10;
    table[b'B' as usize] = 11;
    table[b'C' as usize] = 12;
    table[b'D' as usize] = 13;
    table[b'E' as usize] = 14;
    table[b'F' as usize] = 15;
    table[b'G' as usize] = 16;
    table[b'H' as usize] = 17;
    // I is excluded (maps to 1)
    table[b'J' as usize] = 18;
    table[b'K' as usize] = 19;
    // L is excluded (maps to 1)
    table[b'M' as usize] = 20;
    table[b'N' as usize] = 21;
    // O is excluded (maps to 0)
    table[b'P' as usize] = 22;
    table[b'Q' as usize] = 23;
    table[b'R' as usize] = 24;
    table[b'S' as usize] = 25;
    table[b'T' as usize] = 26;
    // U is excluded
    table[b'V' as usize] = 27;
    table[b'W' as usize] = 28;
    table[b'X' as usize] = 29;
    table[b'Y' as usize] = 30;
    table[b'Z' as usize] = 31;

    // Lowercase → same values
    table[b'a' as usize] = 10;
    table[b'b' as usize] = 11;
    table[b'c' as usize] = 12;
    table[b'd' as usize] = 13;
    table[b'e' as usize] = 14;
    table[b'f' as usize] = 15;
    table[b'g' as usize] = 16;
    table[b'h' as usize] = 17;
    table[b'j' as usize] = 18;
    table[b'k' as usize] = 19;
    table[b'm' as usize] = 20;
    table[b'n' as usize] = 21;
    table[b'p' as usize] = 22;
    table[b'q' as usize] = 23;
    table[b'r' as usize] = 24;
    table[b's' as usize] = 25;
    table[b't' as usize] = 26;
    table[b'v' as usize] = 27;
    table[b'w' as usize] = 28;
    table[b'x' as usize] = 29;
    table[b'y' as usize] = 30;
    table[b'z' as usize] = 31;

    // Common substitutions
    table[b'O' as usize] = 0; // O → 0
    table[b'o' as usize] = 0; // o → 0
    table[b'I' as usize] = 1; // I → 1
    table[b'i' as usize] = 1; // i → 1
    table[b'L' as usize] = 1; // L → 1
    table[b'l' as usize] = 1; // l → 1

    table
};

/// Encode a byte slice to a Crockford Base32 string.
///
/// The output length is `ceil(input_bits / 5)` characters.
/// For 10 bytes (80 bits), output is exactly 16 characters.
pub fn encode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    let bit_len = bytes.len() * 8;
    let out_len = (bit_len + 4) / 5; // ceil(bits / 5)
    let mut result = String::with_capacity(out_len);

    // Process 5 bits at a time from the bit stream
    for i in 0..out_len {
        let bit_offset = i * 5;
        let byte_idx = bit_offset / 8;
        let bit_idx = bit_offset % 8;

        let value = if bit_idx <= 3 {
            // All 5 bits fit in one byte
            (bytes[byte_idx] >> (3 - bit_idx)) & 0x1F
        } else {
            // 5 bits span two bytes
            let high = bytes[byte_idx] << (bit_idx - 3);
            let low = if byte_idx + 1 < bytes.len() {
                bytes[byte_idx + 1] >> (11 - bit_idx)
            } else {
                0
            };
            (high | low) & 0x1F
        };

        result.push(ENCODE_ALPHABET[value as usize] as char);
    }

    result
}

/// Decode a Crockford Base32 string back to bytes.
///
/// Hyphens and spaces are silently stripped (Crockford spec allows them).
/// Returns `Err` if any non-separator character is invalid.
pub fn decode(input: &str) -> Result<Vec<u8>, Base32Error> {
    // Strip separators
    let clean: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'-' && b != b' ')
        .collect();

    if clean.is_empty() {
        return Ok(Vec::new());
    }

    let bit_len = clean.len() * 5;
    let byte_len = bit_len / 8;
    let mut bytes = vec![0u8; byte_len];

    for (i, &ch) in clean.iter().enumerate() {
        if ch >= 128 {
            return Err(Base32Error::InvalidChar(ch as char));
        }
        let value = DECODE_TABLE[ch as usize];
        if value == 255 {
            return Err(Base32Error::InvalidChar(ch as char));
        }

        // Write 5 bits at position i*5 in the output bit stream
        let bit_offset = i * 5;
        let byte_idx = bit_offset / 8;
        let bit_idx = bit_offset % 8;

        if bit_idx <= 3 {
            // All 5 bits fit in one byte
            if byte_idx < byte_len {
                bytes[byte_idx] |= value << (3 - bit_idx);
            }
        } else {
            // 5 bits span two bytes
            if byte_idx < byte_len {
                bytes[byte_idx] |= value >> (bit_idx - 3);
            }
            if byte_idx + 1 < byte_len {
                bytes[byte_idx + 1] |= value << (11 - bit_idx);
            }
        }
    }

    Ok(bytes)
}

/// Errors from Base32 decode.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Base32Error {
    #[error("invalid base32 character: '{0}'")]
    InvalidChar(char),

    #[error("invalid length: expected {expected} bytes, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(&[]), "");
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn encode_10_bytes_gives_16_chars() {
        let bytes = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99];
        let encoded = encode(&bytes);
        assert_eq!(encoded.len(), 16);
        for ch in encoded.chars() {
            assert!(
                "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(ch),
                "unexpected char: {}",
                ch
            );
        }
    }

    #[test]
    fn roundtrip_10_bytes() {
        let original = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB];
        let encoded = encode(&original);
        assert_eq!(encoded.len(), 16);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_all_zeros() {
        let original = [0u8; 10];
        let encoded = encode(&original);
        assert_eq!(encoded, "0000000000000000");
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_all_ones() {
        let original = [0xFF; 10];
        let encoded = encode(&original);
        assert_eq!(encoded, "ZZZZZZZZZZZZZZZZ");
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_strips_hyphens() {
        let original = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB];
        let encoded = encode(&original);
        let with_hyphens = format!(
            "{}-{}-{}-{}",
            &encoded[0..4],
            &encoded[4..8],
            &encoded[8..12],
            &encoded[12..16]
        );
        let decoded = decode(&with_hyphens).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_case_insensitive() {
        let original = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB];
        let encoded = encode(&original);
        let lower = encoded.to_lowercase();
        let decoded = decode(&lower).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_common_substitutions() {
        let decoded_o = decode("O").unwrap();
        let decoded_0 = decode("0").unwrap();
        assert_eq!(decoded_o, decoded_0);

        let decoded_i = decode("I").unwrap();
        let decoded_1 = decode("1").unwrap();
        assert_eq!(decoded_i, decoded_1);

        let decoded_l = decode("L").unwrap();
        assert_eq!(decoded_l, decoded_1);
    }

    #[test]
    fn decode_invalid_char() {
        let err = decode("U").unwrap_err();
        assert_eq!(err, Base32Error::InvalidChar('U'));
    }

    #[test]
    fn roundtrip_random_bytes() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let mut bytes = [0u8; 10];
            rng.fill(&mut bytes);
            let encoded = encode(&bytes);
            assert_eq!(encoded.len(), 16);
            let decoded = decode(&encoded).unwrap();
            assert_eq!(decoded, bytes.to_vec());
        }
    }
}
