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
    pub name: String,
    /// Display label (e.g. "Users").
    pub label: String,
    /// URL path segment (e.g. "users").
    pub path: String,
    /// Icon name.
    pub icon: String,
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

    /// Auto-build a ResourceDef from model IR JSON.
    /// Derives name, label, path from the IR's "resource" and "name" fields.
    pub fn from_ir(module: &str, ir: Value) -> Self {
        let name = ir["resource"].as_str().unwrap_or("unknown").to_string();
        let model_name = ir["name"].as_str().unwrap_or("Unknown").to_string();
        let label = pluralize(&model_name);
        let path = pluralize(&name);
        let icon = default_icon(&name);
        let permissions = Self::crud_permissions(module, &name);
        Self { name, label, path, icon, ir, permissions }
    }

    /// Add custom action permissions.
    pub fn with_action(mut self, module: &str, action: &str) -> Self {
        self.permissions.push(format!("{}:{}:{}", module, self.name, action));
        self
    }

    /// Override the icon.
    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = icon.to_string();
        self
    }
}

fn pluralize(s: &str) -> String {
    if s.ends_with('s') || s.ends_with("sh") || s.ends_with("ch") || s.ends_with('x') {
        format!("{}es", s)
    } else if s.ends_with('y') && s.len() > 1 && !s.ends_with("ey") {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}

fn default_icon(resource: &str) -> String {
    match resource {
        "user" => "users",
        "role" => "shield",
        "group" => "layers",
        "policy" => "lock",
        "session" => "clock",
        "provider" => "globe",
        "device" => "monitor",
        "model" => "box",
        "batch" => "package",
        "firmware" => "cpu",
        "license" => "key",
        "task" => "activity",
        _ => "file",
    }.to_string()
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
                resources: vec![
                    ResourceDef::from_ir("auth", json!({"name": "User", "module": "auth", "resource": "user", "fields": []})),
                ],
            }],
        );

        assert_eq!(schema["name"], "TestApp");
        assert_eq!(schema["modules"][0]["id"], "auth");
        assert_eq!(schema["modules"][0]["hierarchy"]["nav"][0]["label"], "Users");

        let perms = &schema["permissions"]["auth"]["user"];
        assert_eq!(perms.as_array().unwrap().len(), 5);
    }

    #[test]
    fn from_ir_auto_derives() {
        let ir = json!({"name": "Device", "module": "pms", "resource": "device", "fields": []});
        let def = ResourceDef::from_ir("pms", ir);
        assert_eq!(def.name, "device");
        assert_eq!(def.label, "Devices");
        assert_eq!(def.path, "devices");
        assert_eq!(def.icon, "monitor");
        assert_eq!(def.permissions.len(), 5);

        let def = def.with_action("pms", "provision").with_action("pms", "activate");
        assert_eq!(def.permissions.len(), 7);
    }
}
