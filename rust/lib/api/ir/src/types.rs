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

/// UI widget hint — tells the frontend what input component to render.
///
/// Default mapping from Rust types:
///   String         → Text
///   bool           → Switch
///   u32/u64/i32/i64 → Number
///   Vec<String>    → Tags
///   Enum(name)     → Select
///   Option<T>      → same widget as T, but optional
///
/// Custom newtypes override via `#[ui(widget = "...")]`:
///   Avatar         → ImageUpload
///   Url            → UrlInput
///   Email          → EmailInput
///   Password       → PasswordInput
///   Markdown       → MarkdownEditor
///   Color          → ColorPicker
///   DateTime       → DateTimePicker
///   Json           → CodeEditor
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiWidget {
    /// Single-line text input (default for String).
    Text,
    /// Multi-line text area.
    Textarea,
    /// Numeric input with step buttons.
    Number,
    /// Toggle switch (default for bool).
    Switch,
    /// Checkbox.
    Checkbox,
    /// Dropdown select (default for Enum types).
    Select,
    /// Multi-select / tag input (default for Vec<String>).
    Tags,
    /// Email input with validation.
    Email,
    /// URL input with validation.
    Url,
    /// Password input (masked).
    Password,
    /// Image upload / avatar picker.
    ImageUpload,
    /// File upload.
    FileUpload,
    /// Date picker.
    Date,
    /// Date + time picker.
    DateTime,
    /// Color picker.
    Color,
    /// Markdown editor.
    Markdown,
    /// Code/JSON editor.
    Code,
    /// Hidden field (not shown in form).
    Hidden,
    /// Read-only display (not editable).
    ReadOnly,
}

impl UiWidget {
    /// Infer the default widget from a Rust field type.
    pub fn from_field_type(ty: &FieldType) -> Self {
        match ty {
            FieldType::String => UiWidget::Text,
            FieldType::Bool => UiWidget::Switch,
            FieldType::U32 | FieldType::U64 | FieldType::I32 | FieldType::I64 | FieldType::F64 => {
                UiWidget::Number
            }
            FieldType::Option(inner) => Self::from_field_type(inner),
            FieldType::Vec(inner) => match inner.as_ref() {
                FieldType::String => UiWidget::Tags,
                _ => UiWidget::Code, // Vec of complex types -> JSON editor
            },
            FieldType::Enum(_) => UiWidget::Select,
            FieldType::Struct(_) => UiWidget::Code,
            FieldType::Json => UiWidget::Code,
        }
    }

    /// Infer widget from a well-known newtype name (openerp_types::*).
    pub fn from_type_name(name: &str) -> Option<Self> {
        match name {
            // Identity
            "Id" => Some(UiWidget::ReadOnly),
            // Contact
            "Email" | "EmailAddress" => Some(UiWidget::Email),
            "Phone" => Some(UiWidget::Text), // Could be a tel input
            // URLs
            "Url" | "Link" => Some(UiWidget::Url),
            "Avatar" | "ImageUrl" => Some(UiWidget::ImageUpload),
            // Secrets
            "Password" => Some(UiWidget::Password),
            "PasswordHash" | "Secret" => Some(UiWidget::Hidden),
            // Text
            "Text" => Some(UiWidget::Textarea),
            "Markdown" | "RichText" => Some(UiWidget::Markdown),
            "Code" | "JsonData" => Some(UiWidget::Code),
            // Date/time
            "DateTime" | "Timestamp" => Some(UiWidget::DateTime),
            "Date" => Some(UiWidget::Date),
            // Misc
            "Color" | "HexColor" => Some(UiWidget::Color),
            "SemVer" => Some(UiWidget::Text),
            _ => None,
        }
    }
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
