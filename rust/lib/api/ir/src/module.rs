//! Module IR — aggregates all layers for a complete module definition.

use serde::{Deserialize, Serialize};

use crate::db::PersistentIR;
use crate::facet::FacetIR;
use crate::hierarchy::HierarchyIR;
use crate::model::ModelIR;

/// Complete module definition, aggregating all five layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleIR {
    /// Module ID (e.g. `auth`, `pms`, `task`).
    pub id: String,

    /// Display label.
    pub label: String,

    /// Icon name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// All model definitions in this module.
    pub models: Vec<ModelIR>,

    /// Persistent (DB) definitions for each model.
    pub persistent: Vec<PersistentIR>,

    /// Resource hierarchy (nesting / routes / navigation).
    pub hierarchy: HierarchyIR,

    /// API facets (data, app, gear, etc.).
    pub facets: Vec<FacetIR>,
}

impl ModuleIR {
    /// Find a model by name.
    pub fn model(&self, name: &str) -> Option<&ModelIR> {
        self.models.iter().find(|m| m.name == name)
    }

    /// Find a persistent definition for a model.
    pub fn persistent_for(&self, model: &str) -> Option<&PersistentIR> {
        self.persistent.iter().find(|p| p.model == model)
    }

    /// Find all facets for a model.
    pub fn facets_for(&self, model: &str) -> Vec<&FacetIR> {
        self.facets.iter().filter(|f| f.model == model).collect()
    }

    /// Collect all unique permission strings across the entire module.
    pub fn all_permissions(&self) -> Vec<String> {
        let mut perms: Vec<String> = Vec::new();
        for model in &self.models {
            for p in model.all_permissions() {
                if !perms.contains(&p) {
                    perms.push(p);
                }
            }
        }
        for facet in &self.facets {
            for p in facet.permissions(&self.id) {
                if !perms.contains(&p) {
                    perms.push(p);
                }
            }
        }
        perms
    }
}

/// Top-level schema: all modules together.
/// This is what gets serialized to `schema.json` for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSchema {
    /// Application name.
    pub name: String,

    /// All modules.
    pub modules: Vec<ModuleIR>,
}

impl AppSchema {
    /// Find a module by ID.
    pub fn module(&self, id: &str) -> Option<&ModuleIR> {
        self.modules.iter().find(|m| m.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbFieldDef, PersistentIR};
    use crate::hierarchy::{HierarchyIR, ResourceNode};
    use crate::model::{FieldDef, KeyDef, ModelIR};
    use crate::types::{FieldType, StoreType};

    #[test]
    fn module_aggregation() {
        let module = ModuleIR {
            id: "auth".into(),
            label: "Authentication".into(),
            icon: Some("shield".into()),
            models: vec![ModelIR {
                name: "User".into(),
                module: "auth".into(),
                key: KeyDef::single("id"),
                fields: vec![FieldDef {
                    name: "id".into(),
                    ty: FieldType::String,
                    doc: None,
                    reference: None,
                    serde_rename: None,
                }],
                methods: vec![],
                doc: None,
            }],
            persistent: vec![PersistentIR {
                model: "User".into(),
                store: StoreType::Kv,
                key: KeyDef::single("id"),
                indexes: vec![],
                fields: vec![DbFieldDef {
                    name: "id".into(),
                    ty: FieldType::String,
                    hidden: false,
                    auto: None,
                }],
            }],
            hierarchy: HierarchyIR {
                module_id: "auth".into(),
                label: "Authentication".into(),
                icon: Some("shield".into()),
                resources: vec![ResourceNode::leaf("User", "/users")],
            },
            facets: vec![],
        };

        assert!(module.model("User").is_some());
        assert!(module.model("Nonexistent").is_none());
        assert!(module.persistent_for("User").is_some());
        assert_eq!(module.all_permissions().len(), 5); // CRUD for User
    }
}
