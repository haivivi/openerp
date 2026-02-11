//! Production-compatible SerialNumber — matches the TypeScript reference implementation.
//!
//! Bit Layout (80 bits = 10 bytes) with interleaved random obfuscation bits.

use rand::Rng;

use super::base32;

/// Base year for the year field.
const BASE_YEAR: u16 = 2024;

/// Options for generating a batch of serial numbers.
#[derive(Debug, Clone)]
pub struct BatchOptions {
    pub manufacturer: u16,
    pub model: u32,
    pub sales_channel: u16,
    pub timestamp: (u16, u8), // (year, week)
}

/// A 10-byte serial number with interleaved bit fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialNumber {
    data: [u8; 10],
}

impl SerialNumber {
    /// Create a SerialNumber from raw bytes.
    /// Forces version = 1 (top 3 bits of byte 0).
    pub fn from_bytes(mut data: [u8; 10]) -> Self {
        data[0] = (data[0] & 0b0001_1111) | 0b0010_0000;
        Self { data }
    }

    pub fn new() -> Self {
        let mut data = [0u8; 10];
        data[0] = 0b0010_0000;
        Self { data }
    }

    pub fn as_bytes(&self) -> &[u8; 10] {
        &self.data
    }

    pub fn version(&self) -> u8 {
        (self.data[0] & 0b1110_0000) >> 5
    }

    pub fn manufacturer(&self) -> u16 {
        let p0 = (self.data[0] & 0b1) as u16;
        let p1 = self.data[1] as u16;
        let p2 = ((self.data[2] & 0b1000_0000) >> 7) as u16;
        (p0 << 9) | (p1 << 1) | p2
    }

    pub fn set_manufacturer(&mut self, v: u16) {
        assert!(v <= 1023, "manufacturer must be between 0 and 1023, got {}", v);
        let p0 = ((v >> 9) & 0b1) as u8;
        let p1 = ((v >> 1) & 0xFF) as u8;
        let p2 = ((v & 0b1) << 7) as u8;
        self.data[0] = (self.data[0] & 0b1111_1110) | p0;
        self.data[1] = p1;
        self.data[2] = (self.data[2] & 0b0111_1111) | p2;
    }

    pub fn year(&self) -> u16 {
        let p2 = (self.data[2] & 0b0111_1000) >> 3;
        BASE_YEAR + p2 as u16
    }

    pub fn set_year(&mut self, v: u16) {
        assert!(
            (BASE_YEAR..=2039).contains(&v),
            "year must be between 2024 and 2039, got {}",
            v
        );
        let offset = (v - BASE_YEAR) as u8;
        self.data[2] = (self.data[2] & 0b1000_0111) | (offset << 3);
    }

    pub fn model(&self) -> u32 {
        let p3 = (self.data[3] & 0b1) as u32;
        let p4 = self.data[4] as u32;
        let p5 = self.data[5] as u32;
        let p6 = ((self.data[6] & 0b1000_0000) >> 7) as u32;
        (p3 << 17) | (p4 << 9) | (p5 << 1) | p6
    }

    pub fn set_model(&mut self, v: u32) {
        assert!(v <= 262143, "model must be between 0 and 262143, got {}", v);
        let p3 = ((v >> 17) & 0b1) as u8;
        let p4 = ((v >> 9) & 0xFF) as u8;
        let p5 = ((v >> 1) & 0xFF) as u8;
        let p6 = ((v & 0b1) << 7) as u8;
        self.data[3] = (self.data[3] & 0b1111_1110) | p3;
        self.data[4] = p4;
        self.data[5] = p5;
        self.data[6] = (self.data[6] & 0b0111_1111) | p6;
    }

    pub fn week(&self) -> u8 {
        (self.data[6] & 0b0111_1110) >> 1
    }

    pub fn set_week(&mut self, v: u8) {
        assert!(v <= 63, "week must be between 0 and 63, got {}", v);
        self.data[6] = (self.data[6] & 0b1000_0001) | (v << 1);
    }

    pub fn sales_channel(&self) -> u16 {
        let p7 = (self.data[7] & 0b0001_1111) as u16;
        let p8 = ((self.data[8] & 0b1111_1000) >> 3) as u16;
        (p7 << 5) | p8
    }

    pub fn set_sales_channel(&mut self, v: u16) {
        assert!(v <= 1023, "salesChannel must be between 0 and 1023, got {}", v);
        let p7 = ((v >> 5) & 0b1_1111) as u8;
        let p8 = ((v & 0b1_1111) << 3) as u8;
        self.data[7] = (self.data[7] & 0b1110_0000) | p7;
        self.data[8] = (self.data[8] & 0b0000_0111) | p8;
    }

    pub fn obfs(&self) -> u32 {
        let p0 = (self.data[0] & 0b0001_1110) as u32;
        let p2 = (self.data[2] & 0b0000_0111) as u32;
        let p3 = (self.data[3] & 0b1111_1110) as u32;
        let p6 = (self.data[6] & 0b0000_0001) as u32;
        let p7 = (self.data[7] & 0b1110_0000) as u32;
        let p8 = (self.data[8] & 0b0000_0111) as u32;
        let p9 = self.data[9] as u32;
        (p0 << 24) | (p2 << 22) | (p3 << 14) | (p6 << 14) | (p7 << 6) | (p8 << 8) | p9
    }

    pub fn crockford_string(&self) -> String {
        base32::encode(&self.data)
    }

    pub fn from_crockford(s: &str) -> Result<Self, super::base32::Base32Error> {
        let bytes = base32::decode(s)?;
        if bytes.len() != 10 {
            return Err(super::base32::Base32Error::InvalidLength {
                expected: 10,
                actual: bytes.len(),
            });
        }
        let mut data = [0u8; 10];
        data.copy_from_slice(&bytes);
        Ok(Self { data })
    }

    pub fn readable_string(&self) -> String {
        format!(
            "YS{}-{:05X}-{:03X}-{:03X}-{:02}{:02}-{:08X}",
            self.version(),
            self.model(),
            self.manufacturer(),
            self.sales_channel(),
            self.year() % 100,
            self.week(),
            self.obfs()
        )
    }

    pub fn generate(opts: &BatchOptions, count: usize) -> Vec<Self> {
        let mut rng = rand::thread_rng();
        let mut results = Vec::with_capacity(count);
        let mut seen = std::collections::HashSet::new();

        while results.len() < count {
            let mut data = [0u8; 10];
            rng.fill(&mut data);

            let mut sn = Self::from_bytes(data);
            sn.set_manufacturer(opts.manufacturer);
            sn.set_model(opts.model);
            sn.set_year(opts.timestamp.0);
            sn.set_week(opts.timestamp.1);
            sn.set_sales_channel(opts.sales_channel);

            let key = sn.crockford_string();
            if seen.insert(key) {
                results.push(sn);
            }
        }

        results
    }

    pub fn utc_week_of_year(year: u16, month: u8, day: u8) -> u8 {
        if month == 0 || month > 12 || day == 0 {
            return 0;
        }

        let days_in_months: [u16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;

        let mut day_of_year: u16 = 0;
        for i in 0..(month as usize - 1) {
            day_of_year += days_in_months[i];
            if i == 1 && is_leap {
                day_of_year += 1;
            }
        }
        day_of_year += day as u16;

        let jan1_dow = day_of_week(year, 1, 1);
        let week = (day_of_year as i32 + jan1_dow as i32 - 2) / 7;
        week.max(0) as u8
    }
}

fn day_of_week(year: u16, month: u8, day: u8) -> u8 {
    if month == 0 || month > 12 {
        return 0;
    }
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year as i32;
    if month < 3 {
        y -= 1;
    }
    let dow = (y + y / 4 - y / 100 + y / 400 + t[(month - 1) as usize] + day as i32) % 7;
    dow as u8
}

impl Default for SerialNumber {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SerialNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.readable_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_version_1() {
        assert_eq!(SerialNumber::new().version(), 1);
    }

    #[test]
    fn from_bytes_forces_version_1() {
        let sn = SerialNumber::from_bytes([0xFF; 10]);
        assert_eq!(sn.version(), 1);
    }

    #[test]
    fn manufacturer_roundtrip() {
        let mut sn = SerialNumber::new();
        for v in [0, 1, 100, 511, 512, 1023] {
            sn.set_manufacturer(v);
            assert_eq!(sn.manufacturer(), v);
        }
    }

    #[test]
    fn year_roundtrip() {
        let mut sn = SerialNumber::new();
        for y in 2024..=2039 {
            sn.set_year(y);
            assert_eq!(sn.year(), y);
        }
    }

    #[test]
    fn model_roundtrip() {
        let mut sn = SerialNumber::new();
        for v in [0, 1, 255, 256, 65535, 131072, 262143] {
            sn.set_model(v);
            assert_eq!(sn.model(), v);
        }
    }

    #[test]
    fn week_roundtrip() {
        let mut sn = SerialNumber::new();
        for v in 0..=63 {
            sn.set_week(v);
            assert_eq!(sn.week(), v);
        }
    }

    #[test]
    fn sales_channel_roundtrip() {
        let mut sn = SerialNumber::new();
        for v in [0, 1, 100, 511, 512, 1023] {
            sn.set_sales_channel(v);
            assert_eq!(sn.sales_channel(), v);
        }
    }

    #[test]
    fn crockford_string_is_16_chars() {
        assert_eq!(SerialNumber::new().crockford_string().len(), 16);
    }

    #[test]
    fn ts_golden_known_bytes() {
        let sn = SerialNumber::from_bytes(
            [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB],
        );
        assert_eq!(sn.as_bytes()[0], 0x3E);
        assert_eq!(sn.version(), 1);
        assert_eq!(sn.manufacturer(), 347);
        assert_eq!(sn.year(), 2031);
        assert_eq!(sn.model(), 131654);
        assert_eq!(sn.week(), 34);
        assert_eq!(sn.sales_channel(), 241);
        assert_eq!(sn.crockford_string(), "7TPVXVR14D2PF2DB");
        assert_eq!(sn.readable_string(), "YS1-20246-15B-0F1-3134-1FBBD9AB");
    }

    #[test]
    fn ts_golden_set_values() {
        let mut sn = SerialNumber::new();
        sn.set_manufacturer(0x234);
        sn.set_model(0x5678);
        sn.set_year(2025);
        sn.set_week(29);
        sn.set_sales_channel(9);
        let expected: [u8; 10] = [0x21, 0x1A, 0x08, 0x00, 0x2B, 0x3C, 0x3A, 0x00, 0x48, 0x00];
        assert_eq!(sn.as_bytes(), &expected);
        assert_eq!(sn.crockford_string(), "44D0G01B7GX00J00");
        assert_eq!(sn.readable_string(), "YS1-05678-234-009-2529-00000000");
    }

    #[test]
    fn field_independence() {
        let mut sn = SerialNumber::new();
        sn.set_manufacturer(0x3FF);
        sn.set_model(0x3FFFF);
        sn.set_year(2039);
        sn.set_week(63);
        sn.set_sales_channel(0x3FF);
        assert_eq!(sn.manufacturer(), 0x3FF);
        assert_eq!(sn.model(), 0x3FFFF);
        assert_eq!(sn.year(), 2039);
        assert_eq!(sn.week(), 63);
        assert_eq!(sn.sales_channel(), 0x3FF);
    }

    #[test]
    fn crockford_roundtrip() {
        let opts = BatchOptions {
            manufacturer: 42,
            model: 12345,
            sales_channel: 7,
            timestamp: (2025, 15),
        };
        let sns = SerialNumber::generate(&opts, 50);
        for sn in &sns {
            let encoded = sn.crockford_string();
            let decoded = SerialNumber::from_crockford(&encoded).unwrap();
            assert_eq!(sn.data, decoded.data);
        }
    }

    #[test]
    fn generate_unique() {
        let opts = BatchOptions {
            manufacturer: 0x234,
            model: 0x5678,
            sales_channel: 0x9,
            timestamp: (2025, 29),
        };
        let got = SerialNumber::generate(&opts, 100);
        let mut uniq = std::collections::HashSet::new();
        for sn in &got {
            uniq.insert(sn.crockford_string());
        }
        assert_eq!(uniq.len(), 100);
    }
}
