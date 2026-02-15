//! `widget!` macro for declaring UI widget overrides.
//!
//! Usage:
//! ```ignore
//! widget!(textarea { rows: 3, placeholder: "Brief description" }
//!     => [Role.description, Group.description]);
//!
//! widget!(permission_picker { source: "schema.permissions", layout: "modal" }
//!     => [Role.permissions]);
//!
//! widget!(password => [User.password_hash, Provider.client_secret]);
//! ```

/// Declare a widget override.
///
/// Syntax:
///   `widget!(name { key: value, ... } => [Model.field, ...])`
///   `widget!(name => [Model.field, ...])` (no params)
#[macro_export]
macro_rules! widget {
    // With params: widget!(name { k: v, ... } => [targets])
    ($widget:ident { $($key:ident : $val:expr),* $(,)? } => [ $($model:ident . $field:ident),+ $(,)? ]) => {
        $crate::WidgetOverride {
            widget: stringify!($widget).to_string(),
            apply_to: vec![ $( concat!(stringify!($model), ".", stringify!($field)).to_string() ),+ ],
            params: serde_json::json!({ $( stringify!($key): $val ),* }),
        }
    };

    // No params: widget!(name => [targets])
    ($widget:ident => [ $($model:ident . $field:ident),+ $(,)? ]) => {
        $crate::WidgetOverride {
            widget: stringify!($widget).to_string(),
            apply_to: vec![ $( concat!(stringify!($model), ".", stringify!($field)).to_string() ),+ ],
            params: serde_json::Value::Null,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn widget_macro_with_params() {
        let w = widget!(textarea { rows: 3, placeholder: "test" }
            => [Role.description, Group.description]);
        assert_eq!(w.widget, "textarea");
        assert_eq!(w.apply_to, vec!["Role.description", "Group.description"]);
        assert_eq!(w.params["rows"], 3);
        assert_eq!(w.params["placeholder"], "test");
    }

    #[test]
    fn widget_macro_no_params() {
        let w = widget!(permission_picker => [Role.permissions]);
        assert_eq!(w.widget, "permission_picker");
        assert_eq!(w.apply_to, vec!["Role.permissions"]);
        assert!(w.params.is_null());
    }

    #[test]
    fn widget_macro_single_target() {
        let w = widget!(password => [User.password_hash]);
        assert_eq!(w.widget, "password");
        assert_eq!(w.apply_to.len(), 1);
    }

    #[test]
    fn widget_macro_with_string_params() {
        let w = widget!(select {
            source: "/admin/pms/models",
            display: "series_name",
            value: "code"
        } => [Device.model, Batch.model]);
        assert_eq!(w.widget, "select");
        assert_eq!(w.params["source"], "/admin/pms/models");
        assert_eq!(w.apply_to.len(), 2);
    }
}
