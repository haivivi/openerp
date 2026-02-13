//! Semantic newtypes for the OpenERP DSL.
//!
//! These types wrap `String` but carry semantic meaning:
//! - The DSL macro reads the type name to infer UI widget
//! - The validator can apply type-specific validation rules
//! - Serde serializes/deserializes as plain strings (transparent)
//!
//! Usage in model definitions:
//! ```ignore
//! use openerp_types::*;
//!
//! #[model(module = "auth")]
//! #[key(id)]
//! pub struct User {
//!     pub id: Id,
//!     pub name: String,
//!     pub email: Option<Email>,
//!     pub avatar: Option<Avatar>,
//!     pub redirect_url: Url,
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

/// Macro to define a newtype wrapper around String.
macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_inner(self) -> String {
                self.0
            }

            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(String::new())
            }
        }
    };
}

// ── Identity types ──

string_newtype!(
    /// Unique identifier (UUID v4, no dashes).
    /// UI: read-only text, auto-generated.
    Id
);

// ── Contact types ──

string_newtype!(
    /// Email address.
    /// UI: email input with validation.
    Email
);

string_newtype!(
    /// Phone number (E.164 format).
    /// UI: tel input.
    Phone
);

// ── URL types ──

string_newtype!(
    /// A URL / hyperlink.
    /// UI: url input with validation.
    Url
);

string_newtype!(
    /// Avatar image URL.
    /// UI: image upload / preview.
    Avatar
);

string_newtype!(
    /// Generic image URL.
    /// UI: image upload / preview.
    ImageUrl
);

// ── Secret types ──

string_newtype!(
    /// A password (plaintext, for input only — never stored as-is).
    /// UI: password input (masked). Never returned in API responses.
    Password
);

string_newtype!(
    /// A hashed password (argon2/bcrypt). Stored in DB.
    /// UI: hidden (never shown).
    PasswordHash
);

string_newtype!(
    /// A secret token or API key.
    /// UI: password input. Masked in API responses.
    Secret
);

// ── Text types ──

string_newtype!(
    /// Multi-line text / description.
    /// UI: textarea.
    Text
);

string_newtype!(
    /// Markdown-formatted content.
    /// UI: markdown editor.
    Markdown
);

string_newtype!(
    /// Code / JSON content.
    /// UI: code editor with syntax highlighting.
    Code
);

// ── Date/time types ──

string_newtype!(
    /// RFC 3339 datetime string.
    /// UI: datetime picker.
    DateTime
);

string_newtype!(
    /// Date string (YYYY-MM-DD).
    /// UI: date picker.
    Date
);

// ── Misc types ──

string_newtype!(
    /// CSS hex color value (e.g. #ff0000).
    /// UI: color picker.
    Color
);

string_newtype!(
    /// Semantic version string (e.g. 1.2.3).
    /// UI: text input with version format hint.
    SemVer
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_serde() {
        let email = Email::new("alice@test.com");
        let json = serde_json::to_string(&email).unwrap();
        assert_eq!(json, "\"alice@test.com\"");

        let back: Email = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "alice@test.com");
    }

    #[test]
    fn option_serde() {
        let avatar: Option<Avatar> = Some(Avatar::new("https://img.test/a.png"));
        let json = serde_json::to_string(&avatar).unwrap();
        assert_eq!(json, "\"https://img.test/a.png\"");

        let none: Option<Avatar> = None;
        let json = serde_json::to_string(&none).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn deref_and_display() {
        let url = Url::new("https://example.com");
        assert_eq!(url.len(), 19); // Deref to str
        assert_eq!(format!("{}", url), "https://example.com");
    }

    #[test]
    fn default_is_empty() {
        assert!(Id::default().is_empty());
        assert!(Email::default().is_empty());
    }

    #[test]
    fn from_conversions() {
        let e: Email = "test@test.com".into();
        assert_eq!(e.as_str(), "test@test.com");

        let e2: Email = String::from("foo@bar.com").into();
        assert_eq!(e2.as_str(), "foo@bar.com");
    }
}
