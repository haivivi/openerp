//! Model IR — data structure + method signature definitions.
//!
//! Corresponds to `dsl/model/*.rs` files.

use serde::{Deserialize, Serialize};

use crate::types::{FieldType, HttpMethod};

/// A field in a model struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name (e.g. `series_name`).
    pub name: String,

    /// Field type.
    pub ty: FieldType,

    /// Documentation comment, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,

    /// Reference to another model's field for foreign key relationships.
    /// Format: `#[ref(ModelName)]` on the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<FieldRef>,

    /// Serde rename, if any (e.g. `#[serde(rename = "type")]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serde_rename: Option<String>,
}

/// Foreign key reference from a field to another model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRef {
    /// Target model name (e.g. `Model`).
    pub target: String,

    /// GET endpoint to fetch one item (e.g. `/pms/models/{}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<String>,

    /// LIST endpoint to fetch options (e.g. `/pms/models`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,

    /// Which field on the target to use as the value (e.g. `code`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Which field on the target to display in UI (e.g. `series_name`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// Primary/composite key definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyDef {
    /// Field names forming the key. Single element = simple key, multiple = compound key.
    pub fields: Vec<String>,
}

impl KeyDef {
    pub fn single(field: impl Into<String>) -> Self {
        Self {
            fields: vec![field.into()],
        }
    }

    pub fn compound(fields: Vec<String>) -> Self {
        Self { fields }
    }

    pub fn is_compound(&self) -> bool {
        self.fields.len() > 1
    }
}

/// A method signature declared on a model.
///
/// Methods with signatures need hand-written implementations.
/// Models without explicit methods get auto-generated CRUD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodSig {
    /// Method name (e.g. `provision`).
    pub name: String,

    /// HTTP method for this action.
    pub http_method: HttpMethod,

    /// URL path suffix (e.g. `/@provision`).
    pub path: String,

    /// Permission string (e.g. `pms:batch:provision`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<String>,

    /// Parameter definitions.
    pub params: Vec<ParamDef>,

    /// Return type name (e.g. `Batch`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,

    /// Documentation comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// A parameter in a method signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub ty: FieldType,
    pub source: ParamSource,
}

/// Where a parameter comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamSource {
    /// From URL path segment.
    Path,
    /// From request body (JSON).
    Body,
    /// From query string.
    Query,
}

/// Complete model IR: struct definition + optional method signatures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelIR {
    /// Model name (e.g. `User`, `Batch`).
    pub name: String,

    /// Module this model belongs to (e.g. `auth`, `pms`).
    pub module: String,

    /// Primary/composite key.
    pub key: KeyDef,

    /// Struct fields.
    pub fields: Vec<FieldDef>,

    /// Custom method signatures. Empty = auto CRUD only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<MethodSig>,

    /// Documentation comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

impl ModelIR {
    /// Get a field by name.
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get the key field(s).
    pub fn key_fields(&self) -> Vec<&FieldDef> {
        self.key
            .fields
            .iter()
            .filter_map(|k| self.field(k))
            .collect()
    }

    /// Auto-generated permission strings for standard CRUD.
    pub fn crud_permissions(&self) -> Vec<String> {
        let resource = to_snake_case(&self.name);
        vec![
            format!("{}:{}:create", self.module, resource),
            format!("{}:{}:read", self.module, resource),
            format!("{}:{}:update", self.module, resource),
            format!("{}:{}:delete", self.module, resource),
            format!("{}:{}:list", self.module, resource),
        ]
    }

    /// All permissions: CRUD + custom methods.
    pub fn all_permissions(&self) -> Vec<String> {
        let mut perms = self.crud_permissions();
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

    fn sample_user_model() -> ModelIR {
        ModelIR {
            name: "User".into(),
            module: "auth".into(),
            key: KeyDef::single("id"),
            fields: vec![
                FieldDef {
                    name: "id".into(),
                    ty: FieldType::String,
                    doc: None,
                    reference: None,
                    serde_rename: None,
                },
                FieldDef {
                    name: "name".into(),
                    ty: FieldType::String,
                    doc: None,
                    reference: None,
                    serde_rename: None,
                },
                FieldDef {
                    name: "email".into(),
                    ty: FieldType::Option(Box::new(FieldType::String)),
                    doc: None,
                    reference: None,
                    serde_rename: None,
                },
            ],
            methods: vec![],
            doc: None,
        }
    }

    #[test]
    fn crud_permissions() {
        let model = sample_user_model();
        let perms = model.crud_permissions();
        assert_eq!(
            perms,
            vec![
                "auth:user:create",
                "auth:user:read",
                "auth:user:update",
                "auth:user:delete",
                "auth:user:list",
            ]
        );
    }

    #[test]
    fn key_fields() {
        let model = sample_user_model();
        let keys = model.key_fields();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "id");
    }

    #[test]
    fn compound_key() {
        let model = ModelIR {
            name: "Firmware".into(),
            module: "pms".into(),
            key: KeyDef::compound(vec!["model".into(), "semver".into()]),
            fields: vec![
                FieldDef {
                    name: "model".into(),
                    ty: FieldType::U32,
                    doc: None,
                    reference: None,
                    serde_rename: None,
                },
                FieldDef {
                    name: "semver".into(),
                    ty: FieldType::String,
                    doc: None,
                    reference: None,
                    serde_rename: None,
                },
            ],
            methods: vec![],
            doc: None,
        };
        assert!(model.key.is_compound());
        assert_eq!(model.key_fields().len(), 2);
    }

    #[test]
    fn serde_roundtrip() {
        let model = sample_user_model();
        let json = serde_json::to_string_pretty(&model).unwrap();
        let back: ModelIR = serde_json::from_str(&json).unwrap();
        assert_eq!(model, back);
    }
}
