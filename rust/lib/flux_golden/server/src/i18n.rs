//! Internationalization stub.
//!
//! Provides a `Localizer` trait for all user-facing strings.
//! Default implementation returns original text (English).
//! Replace with a real i18n library (e.g. fluent, gettext) later.
//!
//! Golden test: establishes the pattern. Codegen will generate
//! localization keys from #[state] and #[request] definitions.

use std::sync::Arc;

/// Localizer trait — translates keys to localized strings.
///
/// All user-facing error messages and UI labels go through this.
/// Implementations can load translations from files, databases, etc.
pub trait Localizer: Send + Sync + 'static {
    /// Translate a key to a localized string.
    ///
    /// `key` is a dot-separated identifier: "error.tweet.empty", "label.sign_in".
    /// `args` are named substitutions: [("max", "280")] → "超过{max}字符".
    fn t(&self, key: &str, args: &[(&str, &str)]) -> String;
}

/// Default localizer — returns English text for known keys,
/// or the key itself for unknown keys.
///
/// This is the stub. Replace with a real implementation later.
pub struct DefaultLocalizer;

impl Localizer for DefaultLocalizer {
    fn t(&self, key: &str, args: &[(&str, &str)]) -> String {
        let text = match key {
            // Auth
            "error.auth.missing_token" => "Missing Authorization header",
            "error.auth.invalid_token" => "Invalid or expired token",
            "error.auth.user_not_found" => "User '{username}' not found",

            // Tweet
            "error.tweet.empty" => "Tweet cannot be empty",
            "error.tweet.too_long" => "Tweet exceeds {max} characters",

            // Profile
            "error.profile.name_empty" => "Display name cannot be empty",
            "error.profile.not_found" => "User '{id}' not found",

            // Tweet detail
            "error.tweet.not_found" => "Tweet '{id}' not found",

            // Password
            "error.password.too_short" => "Password must be at least {min} characters",
            "error.password.same" => "New password must be different from old password",

            // Generic
            "error.internal" => "Internal server error",

            // Unknown key — return the key itself.
            _ => return key.to_string(),
        };

        // Substitute args: {name} → value.
        let mut result = text.to_string();
        for (name, value) in args {
            result = result.replace(&format!("{{{}}}", name), value);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_key_returns_english() {
        let l = DefaultLocalizer;
        assert_eq!(l.t("error.tweet.empty", &[]), "Tweet cannot be empty");
    }

    #[test]
    fn args_substituted() {
        let l = DefaultLocalizer;
        assert_eq!(
            l.t("error.tweet.too_long", &[("max", "280")]),
            "Tweet exceeds 280 characters"
        );
    }

    #[test]
    fn multiple_args() {
        let l = DefaultLocalizer;
        assert_eq!(
            l.t("error.auth.user_not_found", &[("username", "alice")]),
            "User 'alice' not found"
        );
    }

    #[test]
    fn unknown_key_returns_key() {
        let l = DefaultLocalizer;
        assert_eq!(l.t("some.unknown.key", &[]), "some.unknown.key");
    }
}
