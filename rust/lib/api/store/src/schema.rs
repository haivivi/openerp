//! Schema builder — collects model IR into a JSON schema for the frontend.
//!
//! Each `#[model]` has `__dsl_ir()` that returns serde_json::Value.
//! This module provides helpers to aggregate them into the full schema
//! served at `/meta/schema`.

use serde_json::{json, Value};

/// A module definition for the schema.
pub struct ModuleDef {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub resources: Vec<ResourceDef>,
}

/// A resource definition within a module.
pub struct ResourceDef {
    /// Snake_case resource name (e.g. "user").
    pub name: &'static str,
    /// Display label (e.g. "Users").
    pub label: &'static str,
    /// URL path segment (e.g. "users").
    pub path: &'static str,
    /// Icon name.
    pub icon: &'static str,
    /// Model IR (from __dsl_ir()).
    pub ir: Value,
    /// Permissions for this resource (CRUD + custom actions).
    pub permissions: Vec<String>,
}

impl ResourceDef {
    /// Build standard CRUD permissions for a resource.
    pub fn crud_permissions(module: &str, resource: &str) -> Vec<String> {
        vec![
            format!("{}:{}:create", module, resource),
            format!("{}:{}:read", module, resource),
            format!("{}:{}:update", module, resource),
            format!("{}:{}:delete", module, resource),
            format!("{}:{}:list", module, resource),
        ]
    }
}

/// Build the complete schema JSON from module definitions.
pub fn build_schema(app_name: &str, modules: Vec<ModuleDef>) -> Value {
    let mut all_permissions = serde_json::Map::new();

    let module_values: Vec<Value> = modules
        .iter()
        .map(|m| {
            // Collect permissions per module.
            let mut mod_perms = serde_json::Map::new();
            for r in &m.resources {
                mod_perms.insert(
                    r.name.to_string(),
                    Value::Array(
                        r.permissions
                            .iter()
                            .map(|p| Value::String(p.clone()))
                            .collect(),
                    ),
                );
            }
            all_permissions
                .insert(m.id.to_string(), Value::Object(mod_perms));

            // Build module JSON.
            json!({
                "id": m.id,
                "label": m.label,
                "icon": m.icon,
                "resources": m.resources.iter().map(|r| &r.ir).collect::<Vec<_>>(),
                "hierarchy": {
                    "nav": m.resources.iter().map(|r| json!({
                        "model": r.ir["name"],
                        "path": format!("/{}", r.path),
                        "label": r.label,
                        "icon": r.icon,
                    })).collect::<Vec<_>>()
                },
                "facets": ["admin"]
            })
        })
        .collect();

    json!({
        "name": app_name,
        "modules": module_values,
        "permissions": all_permissions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_schema_basic() {
        let schema = build_schema(
            "TestApp",
            vec![ModuleDef {
                id: "auth",
                label: "Authentication",
                icon: "shield",
                resources: vec![ResourceDef {
                    name: "user",
                    label: "Users",
                    path: "users",
                    icon: "users",
                    ir: json!({"name": "User", "module": "auth", "fields": []}),
                    permissions: ResourceDef::crud_permissions("auth", "user"),
                }],
            }],
        );

        assert_eq!(schema["name"], "TestApp");
        assert_eq!(schema["modules"][0]["id"], "auth");
        assert_eq!(schema["modules"][0]["hierarchy"]["nav"][0]["label"], "Users");

        let perms = &schema["permissions"]["auth"]["user"];
        assert_eq!(perms.as_array().unwrap().len(), 5);
    }
}
