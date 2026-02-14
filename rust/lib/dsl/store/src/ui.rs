//! UI widget override definitions.
//!
//! Widget overrides are declared in `dsl/ui/*.rs` files and merged
//! into the schema JSON so the frontend knows how to render each field.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A widget override: applies a widget configuration to one or more fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetOverride {
    /// Widget type (e.g. "permission_picker", "textarea", "select").
    pub widget: String,

    /// Fields this override applies to, in "Model.field" format.
    pub apply_to: Vec<String>,

    /// Additional widget parameters (rows, placeholder, source, etc.).
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

/// Merge widget overrides into the schema JSON.
///
/// For each override, find matching fields in the schema and add/replace
/// the widget + params.
pub fn apply_overrides(schema: &mut Value, overrides: &[WidgetOverride]) {
    let modules = match schema.get_mut("modules").and_then(|m| m.as_array_mut()) {
        Some(m) => m,
        None => return,
    };

    for ov in overrides {
        for target in &ov.apply_to {
            let parts: Vec<&str> = target.split('.').collect();
            if parts.len() != 2 {
                continue;
            }
            let model_name = parts[0];
            let field_name = parts[1];

            // Find the field in the schema and update it.
            for module in modules.iter_mut() {
                if let Some(resources) = module.get_mut("resources").and_then(|r| r.as_array_mut())
                {
                    for resource in resources.iter_mut() {
                        if resource.get("name").and_then(|n| n.as_str()) != Some(model_name) {
                            continue;
                        }
                        if let Some(fields) =
                            resource.get_mut("fields").and_then(|f| f.as_array_mut())
                        {
                            for field in fields.iter_mut() {
                                if field.get("name").and_then(|n| n.as_str())
                                    == Some(field_name)
                                {
                                    // Set widget.
                                    field["widget"] = Value::String(ov.widget.clone());
                                    // Merge params.
                                    if let Some(params_obj) = ov.params.as_object() {
                                        for (k, v) in params_obj {
                                            field[k] = v.clone();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn apply_overrides_basic() {
        let mut schema = json!({
            "modules": [{
                "id": "auth",
                "resources": [{
                    "name": "Role",
                    "fields": [
                        {"name": "id", "widget": "readonly"},
                        {"name": "permissions", "widget": "tags"},
                        {"name": "description", "widget": "text"},
                    ]
                }]
            }]
        });

        let overrides = vec![
            WidgetOverride {
                widget: "permission_picker".into(),
                apply_to: vec!["Role.permissions".into()],
                params: json!({"source": "schema.permissions", "layout": "modal"}),
            },
            WidgetOverride {
                widget: "textarea".into(),
                apply_to: vec!["Role.description".into()],
                params: json!({"rows": 3, "placeholder": "What this role is for"}),
            },
        ];

        apply_overrides(&mut schema, &overrides);

        let fields = &schema["modules"][0]["resources"][0]["fields"];
        // permissions should now be permission_picker with params.
        assert_eq!(fields[1]["widget"], "permission_picker");
        assert_eq!(fields[1]["source"], "schema.permissions");
        assert_eq!(fields[1]["layout"], "modal");
        // description should be textarea with rows.
        assert_eq!(fields[2]["widget"], "textarea");
        assert_eq!(fields[2]["rows"], 3);
        // id unchanged.
        assert_eq!(fields[0]["widget"], "readonly");
    }

    #[test]
    fn apply_to_multiple_fields() {
        let mut schema = json!({
            "modules": [{
                "id": "auth",
                "resources": [
                    {"name": "Role", "fields": [{"name": "description", "widget": "text"}]},
                    {"name": "Group", "fields": [{"name": "description", "widget": "text"}]},
                ]
            }]
        });

        let overrides = vec![WidgetOverride {
            widget: "textarea".into(),
            apply_to: vec!["Role.description".into(), "Group.description".into()],
            params: json!({"rows": 3}),
        }];

        apply_overrides(&mut schema, &overrides);

        assert_eq!(schema["modules"][0]["resources"][0]["fields"][0]["widget"], "textarea");
        assert_eq!(schema["modules"][0]["resources"][1]["fields"][0]["widget"], "textarea");
    }
}
