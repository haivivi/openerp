//! DB (Persistent) IR — storage definitions.
//!
//! Corresponds to `dsl/persistent/*.rs` files.
//! Defines how a model is stored: key, indexes, store type, hidden fields.

use serde::{Deserialize, Serialize};

use crate::types::{FieldType, IndexKind, StoreType};
use crate::model::KeyDef;

/// An index definition on the DB struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
    /// Field name(s) in the index.
    pub fields: Vec<String>,

    /// Index kind (unique, regular, search, filter).
    pub kind: IndexKind,
}

/// A field in the DB struct (superset of model fields — may include hidden fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DbFieldDef {
    /// Field name.
    pub name: String,

    /// Field type.
    pub ty: FieldType,

    /// If true, this field is NOT in the model (hidden from API).
    #[serde(default)]
    pub hidden: bool,

    /// Auto-fill behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<AutoFill>,
}

/// Automatic field fill behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoFill {
    /// Set to current timestamp on create.
    CreateTimestamp,
    /// Set to current timestamp on update.
    UpdateTimestamp,
    /// Generate UUID v4 on create.
    Uuid,
}

/// Complete persistent IR for a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentIR {
    /// Which model this persistent definition is for (e.g. `User`).
    pub model: String,

    /// Storage engine.
    #[serde(default)]
    pub store: StoreType,

    /// Primary key (must match or be a subset of the model's key).
    pub key: KeyDef,

    /// Indexes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<IndexDef>,

    /// All fields in the DB struct (model fields + hidden fields).
    pub fields: Vec<DbFieldDef>,
}

impl PersistentIR {
    /// Fields that are hidden from the API (in DB but not in model).
    pub fn hidden_fields(&self) -> Vec<&DbFieldDef> {
        self.fields.iter().filter(|f| f.hidden).collect()
    }

    /// Fields that have auto-fill behavior.
    pub fn auto_fields(&self) -> Vec<&DbFieldDef> {
        self.fields.iter().filter(|f| f.auto.is_some()).collect()
    }

    /// Get indexes of a specific kind.
    pub fn indexes_of(&self, kind: IndexKind) -> Vec<&IndexDef> {
        self.indexes.iter().filter(|i| i.kind == kind).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_db_hidden_fields() {
        let db = PersistentIR {
            model: "User".into(),
            store: StoreType::Kv,
            key: KeyDef::single("id"),
            indexes: vec![
                IndexDef {
                    fields: vec!["email".into()],
                    kind: IndexKind::Unique,
                },
                IndexDef {
                    fields: vec!["name".into()],
                    kind: IndexKind::Index,
                },
            ],
            fields: vec![
                DbFieldDef {
                    name: "id".into(),
                    ty: FieldType::String,
                    hidden: false,
                    auto: Some(AutoFill::Uuid),
                },
                DbFieldDef {
                    name: "name".into(),
                    ty: FieldType::String,
                    hidden: false,
                    auto: None,
                },
                DbFieldDef {
                    name: "password_hash".into(),
                    ty: FieldType::String,
                    hidden: true,
                    auto: None,
                },
                DbFieldDef {
                    name: "created_at".into(),
                    ty: FieldType::String,
                    hidden: false,
                    auto: Some(AutoFill::CreateTimestamp),
                },
            ],
        };

        assert_eq!(db.hidden_fields().len(), 1);
        assert_eq!(db.hidden_fields()[0].name, "password_hash");
        assert_eq!(db.auto_fields().len(), 2);
        assert_eq!(db.indexes_of(IndexKind::Unique).len(), 1);
    }
}
