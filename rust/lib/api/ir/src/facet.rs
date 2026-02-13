//! Facet IR — REST API surface definitions.
//!
//! Corresponds to `dsl/rest/{facet_name}/*.rs` files.
//! Each facet is a different API surface for a different consumer:
//!   - `data/` — full data access, IAM-controlled
//!   - `app/`  — mobile app API
//!   - `gear/` — hardware device API
//!   - `agent/` — agent system API

use serde::{Deserialize, Serialize};

use crate::model::MethodSig;
use crate::types::{AuthMethod, FieldType};

/// A field exposed in a facet (subset of the model's fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetFieldDef {
    /// Field name (must exist in the source model).
    pub name: String,

    /// Field type (must match the model's field type).
    pub ty: FieldType,

    /// Whether this field is read-only in this facet.
    #[serde(default)]
    pub readonly: bool,
}

/// Complete facet IR for one model in one facet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetIR {
    /// Facet name (e.g. `data`, `app`, `gear`).
    pub facet: String,

    /// URL path prefix for this facet (e.g. `/data`, `/app`).
    pub path: String,

    /// Authentication method.
    #[serde(default)]
    pub auth: AuthMethod,

    /// Source model name (e.g. `User`).
    pub model: String,

    /// Fields exposed in this facet (subset of model fields).
    pub fields: Vec<FacetFieldDef>,

    /// Custom method signatures specific to this facet.
    /// Methods with signatures need hand-written implementations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<MethodSig>,

    /// If true, auto-generate standard CRUD endpoints.
    #[serde(default = "default_true")]
    pub crud: bool,
}

fn default_true() -> bool {
    true
}

impl FacetIR {
    /// Get a field by name.
    pub fn field(&self, name: &str) -> Option<&FacetFieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Field names exposed in this facet.
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    /// All permissions needed by this facet (CRUD + custom methods).
    pub fn permissions(&self, module: &str) -> Vec<String> {
        let mut perms = Vec::new();
        let resource = to_snake_case(&self.model);

        if self.crud {
            perms.push(format!("{}:{}:create", module, resource));
            perms.push(format!("{}:{}:read", module, resource));
            perms.push(format!("{}:{}:update", module, resource));
            perms.push(format!("{}:{}:delete", module, resource));
            perms.push(format!("{}:{}:list", module, resource));
        }

        for m in &self.methods {
            if let Some(p) = &m.permission {
                perms.push(p.clone());
            }
        }

        perms
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facet_field_subset() {
        let facet = FacetIR {
            facet: "data".into(),
            path: "/data".into(),
            auth: AuthMethod::Jwt,
            model: "User".into(),
            fields: vec![
                FacetFieldDef {
                    name: "id".into(),
                    ty: FieldType::String,
                    readonly: true,
                },
                FacetFieldDef {
                    name: "name".into(),
                    ty: FieldType::String,
                    readonly: false,
                },
            ],
            methods: vec![],
            crud: true,
        };

        assert_eq!(facet.field_names(), vec!["id", "name"]);
        assert!(facet.field("id").unwrap().readonly);
        assert!(!facet.field("name").unwrap().readonly);
    }

    #[test]
    fn facet_permissions() {
        let facet = FacetIR {
            facet: "data".into(),
            path: "/data".into(),
            auth: AuthMethod::Jwt,
            model: "User".into(),
            fields: vec![],
            methods: vec![],
            crud: true,
        };

        let perms = facet.permissions("auth");
        assert_eq!(perms.len(), 5);
        assert!(perms.contains(&"auth:user:create".to_string()));
    }
}
