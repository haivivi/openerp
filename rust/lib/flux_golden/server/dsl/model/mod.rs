//! Twitter model definitions.
//!
//! Each model is a single file with `#[model]` — the macro generates
//! serde, Field consts, IR metadata, and common fields.

pub mod user;
pub mod tweet;
pub mod like;
pub mod follow;

pub use user::User;
pub use tweet::Tweet;
pub use like::Like;
pub use follow::Follow;

/// Flexible u32 deserializer — accepts both number and string.
/// Dashboard forms send all values as strings ("0" instead of 0).
pub fn de_u32<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    use serde::Deserialize;
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Number(n) => n.as_u64()
            .map(|n| n as u32)
            .ok_or_else(|| serde::de::Error::custom("invalid number")),
        serde_json::Value::String(s) => s.parse::<u32>()
            .map_err(|_| serde::de::Error::custom(format!("cannot parse '{}' as u32", s))),
        serde_json::Value::Null => Ok(0),
        _ => Err(serde::de::Error::custom("expected number or string")),
    }
}
