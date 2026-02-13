//! Shared types used across all IR layers.

use serde::{Deserialize, Serialize};

/// Rust type representation for fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Bool,
    U32,
    U64,
    I32,
    I64,
    F64,
    /// `Option<inner>`
    Option(Box<FieldType>),
    /// `Vec<inner>`
    Vec(Box<FieldType>),
    /// A named enum type (e.g. `BatchStatus`).
    Enum(String),
    /// A named struct type (e.g. `FirmwareFile`).
    Struct(String),
    /// Opaque JSON (`serde_json::Value`).
    Json,
}

impl FieldType {
    /// Returns the inner type if this is `Option<T>`, otherwise `None`.
    pub fn option_inner(&self) -> Option<&FieldType> {
        match self {
            FieldType::Option(inner) => Some(inner),
            _ => None,
        }
    }

    /// Returns true if this is `Option<T>`.
    pub fn is_optional(&self) -> bool {
        matches!(self, FieldType::Option(_))
    }

    /// Returns true if this is `Vec<T>`.
    pub fn is_vec(&self) -> bool {
        matches!(self, FieldType::Vec(_))
    }
}

/// Storage engine type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreType {
    /// Key-value store (redb). Value is the serialized struct.
    Kv,
    /// SQL store (SQLite). Each indexed field gets a column.
    Sql,
}

impl Default for StoreType {
    fn default() -> Self {
        Self::Kv
    }
}

/// Index type for persistent definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexKind {
    /// Unique constraint.
    Unique,
    /// Regular index for queries.
    Index,
    /// Full-text search index (tantivy).
    Search,
    /// Filter index (for list filtering).
    Filter,
}

/// Authentication method for a facet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// JWT bearer token.
    Jwt,
    /// Device token (hardware devices).
    DeviceToken,
    /// API key.
    ApiKey,
    /// No authentication required.
    None,
    /// Custom authenticator (identified by name).
    Custom(String),
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::Jwt
    }
}

/// HTTP method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_type_optional() {
        let ty = FieldType::Option(Box::new(FieldType::String));
        assert!(ty.is_optional());
        assert_eq!(ty.option_inner(), Some(&FieldType::String));

        assert!(!FieldType::String.is_optional());
        assert!(FieldType::Vec(Box::new(FieldType::U32)).is_vec());
    }

    #[test]
    fn serde_roundtrip() {
        let ty = FieldType::Option(Box::new(FieldType::Enum("BatchStatus".into())));
        let json = serde_json::to_string(&ty).unwrap();
        let back: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(ty, back);
    }
}
