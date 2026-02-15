//! Schema builder — collects model IR into a JSON schema for the frontend.
//!
//! Each `#[model]` has `__dsl_ir()` that returns serde_json::Value.
//! This module provides helpers to aggregate them into the full schema
//! served at `/meta/schema`.

use serde_json::{json, Value};

use crate::hierarchy::HierarchyNode;

/// A module definition for the schema.
pub struct ModuleDef {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub resources: Vec<ResourceDef>,
    /// Resource hierarchy tree for sidebar navigation and detail page relations.
    pub hierarchy: Vec<HierarchyNode>,
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
    /// Short description (from hierarchy doc comments).
    pub description: String,
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
        Self { name, label, path, icon, description: String::new(), ir, permissions }
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

    /// Set description (from hierarchy doc comment).
    pub fn with_desc(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
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

/// Generate a human-readable description for a CRUD action.
fn action_description(action: &str, resource_label: &str) -> String {
    match action {
        "create" => format!("Create {}", resource_label.trim_end_matches('s')),
        "read" => format!("View {} details", resource_label.trim_end_matches('s')),
        "update" => format!("Edit {}", resource_label.trim_end_matches('s')),
        "delete" => format!("Delete {}", resource_label.trim_end_matches('s')),
        "list" => format!("List all {}", resource_label),
        other => format!("{} {}", capitalize(other), resource_label.trim_end_matches('s')),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
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
            // Collect permissions per module with descriptions.
            let mut mod_perms = serde_json::Map::new();
            for r in &m.resources {
                let actions: Vec<Value> = r.permissions.iter().map(|p| {
                    let action = p.rsplit(':').next().unwrap_or(p);
                    let desc = action_description(action, &r.label);
                    json!({ "perm": p, "action": action, "desc": desc })
                }).collect();
                mod_perms.insert(
                    r.name.to_string(),
                    json!({
                        "label": r.label,
                        "description": r.description,
                        "actions": actions
                    }),
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
                    "nav": m.hierarchy.iter().map(|h| h.to_json()).collect::<Vec<_>>()
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
                    ResourceDef::from_ir("auth", json!({"name": "User", "module": "auth", "resource": "user", "fields": []}))
                        .with_desc("User identity management"),
                ],
                hierarchy: vec![
                    HierarchyNode::leaf("user", "Users", "users", "User identity management"),
                ],
            }],
        );

        assert_eq!(schema["name"], "TestApp");
        assert_eq!(schema["modules"][0]["id"], "auth");
        assert_eq!(schema["modules"][0]["hierarchy"]["nav"][0]["label"], "Users");

        let user_perms = &schema["permissions"]["auth"]["user"];
        assert_eq!(user_perms["description"], "User identity management");
        assert_eq!(user_perms["actions"].as_array().unwrap().len(), 5);
        assert_eq!(user_perms["actions"][0]["action"], "create");
        assert!(user_perms["actions"][0]["desc"].as_str().unwrap().contains("User"));
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
