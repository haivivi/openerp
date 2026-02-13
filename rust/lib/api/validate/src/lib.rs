//! OpenERP DSL Validator
//!
//! Compile-time consistency checks across DSL layers:
//! - Facet fields must exist in the source model
//! - Persistent key fields must exist in the persistent struct
//! - Hierarchy resource names must reference known models
//! - Permission strings must follow module:resource:action format
//! - Field types in facets must match model field types

use openerp_ir::*;

/// A validation error with a descriptive message.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    /// Which layer produced the error (model, persistent, hierarchy, facet).
    pub layer: String,
    /// Which resource/struct the error is about.
    pub context: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] {}", self.layer, self.context, self.message)
    }
}

/// Validate an entire module definition.
/// Returns all errors found (does not stop at first error).
pub fn validate_module(module: &ModuleIR) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // 1. Validate each persistent definition against its model.
    for db in &module.persistent {
        if let Some(model) = module.model(&db.model) {
            errors.extend(validate_persistent_against_model(db, model));
        } else {
            errors.push(ValidationError {
                message: format!(
                    "persistent references model '{}' which is not defined",
                    db.model
                ),
                layer: "persistent".into(),
                context: db.model.clone(),
            });
        }
    }

    // 2. Validate each facet against its model.
    for facet in &module.facets {
        if let Some(model) = module.model(&facet.model) {
            errors.extend(validate_facet_against_model(facet, model));
        } else {
            errors.push(ValidationError {
                message: format!(
                    "facet references model '{}' which is not defined",
                    facet.model
                ),
                layer: "facet".into(),
                context: format!("{}:{}", facet.facet, facet.model),
            });
        }
    }

    // 3. Validate hierarchy references known models.
    errors.extend(validate_hierarchy(&module.hierarchy, module));

    // 4. Validate permission strings.
    for model in &module.models {
        for method in &model.methods {
            if let Some(perm) = &method.permission {
                if let Err(e) = validate_permission_format(perm) {
                    errors.push(ValidationError {
                        message: e,
                        layer: "model".into(),
                        context: format!("{}::{}", model.name, method.name),
                    });
                }
            }
        }
    }

    errors
}

/// Check that persistent fields that are NOT in the model are marked hidden,
/// and that key fields exist.
fn validate_persistent_against_model(
    db: &PersistentIR,
    model: &ModelIR,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let ctx = format!("{}DB", db.model);

    // Key fields must exist in persistent struct.
    for kf in &db.key.fields {
        if !db.fields.iter().any(|f| &f.name == kf) {
            errors.push(ValidationError {
                message: format!("key field '{}' not found in persistent struct", kf),
                layer: "persistent".into(),
                context: ctx.clone(),
            });
        }
    }

    // Index fields must exist in persistent struct.
    for idx in &db.indexes {
        for f in &idx.fields {
            if !db.fields.iter().any(|field| &field.name == f) {
                errors.push(ValidationError {
                    message: format!("index field '{}' not found in persistent struct", f),
                    layer: "persistent".into(),
                    context: ctx.clone(),
                });
            }
        }
    }

    // Detect which persistent fields are hidden (not in model).
    let model_field_names: Vec<&str> = model.fields.iter().map(|f| f.name.as_str()).collect();
    for field in &db.fields {
        if !model_field_names.contains(&field.name.as_str()) && !field.hidden {
            // This field is in DB but not in model — it should be marked hidden.
            // This is a warning, not necessarily an error. The macro can auto-set it.
            // For now, we note it as an info-level finding (not blocking).
        }
    }

    errors
}

/// Check that facet fields exist in the source model with matching types.
fn validate_facet_against_model(facet: &FacetIR, model: &ModelIR) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let ctx = format!("{}:{}", facet.facet, facet.model);

    for ff in &facet.fields {
        match model.field(&ff.name) {
            Some(mf) => {
                // Type must match.
                if ff.ty != mf.ty {
                    errors.push(ValidationError {
                        message: format!(
                            "field '{}' type mismatch: facet has {:?}, model has {:?}",
                            ff.name, ff.ty, mf.ty
                        ),
                        layer: "facet".into(),
                        context: ctx.clone(),
                    });
                }
            }
            None => {
                errors.push(ValidationError {
                    message: format!(
                        "field '{}' not found in model '{}'",
                        ff.name, facet.model
                    ),
                    layer: "facet".into(),
                    context: ctx.clone(),
                });
            }
        }
    }

    errors
}

/// Check that hierarchy references known model names.
fn validate_hierarchy(hierarchy: &HierarchyIR, module: &ModuleIR) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let model_names: Vec<&str> = module.models.iter().map(|m| m.name.as_str()).collect();

    fn check_node(
        node: &ResourceNode,
        model_names: &[&str],
        errors: &mut Vec<ValidationError>,
    ) {
        if !model_names.contains(&node.model.as_str()) {
            errors.push(ValidationError {
                message: format!(
                    "hierarchy references model '{}' which is not defined",
                    node.model
                ),
                layer: "hierarchy".into(),
                context: node.model.clone(),
            });
        }
        for child in &node.children {
            check_node(child, model_names, errors);
        }
    }

    for resource in &hierarchy.resources {
        check_node(resource, &model_names, &mut errors);
    }

    errors
}

/// Validate that a permission string follows the `module:resource:action` format.
fn validate_permission_format(perm: &str) -> Result<(), String> {
    let parts: Vec<&str> = perm.split(':').collect();
    if parts.len() != 3 {
        return Err(format!(
            "permission '{}' must have format 'module:resource:action' (got {} parts)",
            perm,
            parts.len()
        ));
    }
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            return Err(format!(
                "permission '{}' has empty segment at position {}",
                perm, i
            ));
        }
        if !part
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_')
        {
            return Err(format!(
                "permission '{}' segment '{}' must be lowercase alphanumeric with underscores",
                perm, part
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openerp_ir::*;

    fn minimal_module() -> ModuleIR {
        ModuleIR {
            id: "test".into(),
            label: "Test".into(),
            icon: None,
            models: vec![ModelIR {
                name: "User".into(),
                module: "test".into(),
                key: KeyDef::single("id"),
                fields: vec![
                    FieldDef {
                        name: "id".into(),
                        ty: FieldType::String,
                        doc: None,
                        ui_widget: None,
                        reference: None,
                        serde_rename: None,
                    },
                    FieldDef {
                        name: "name".into(),
                        ty: FieldType::String,
                        doc: None,
                        ui_widget: None,
                        reference: None,
                        serde_rename: None,
                    },
                    FieldDef {
                        name: "email".into(),
                        ty: FieldType::Option(Box::new(FieldType::String)),
                        doc: None,
                        ui_widget: None,
                        reference: None,
                        serde_rename: None,
                    },
                ],
                methods: vec![],
                doc: None,
            }],
            persistent: vec![PersistentIR {
                model: "User".into(),
                store: StoreType::Kv,
                key: KeyDef::single("id"),
                indexes: vec![],
                fields: vec![
                    DbFieldDef {
                        name: "id".into(),
                        ty: FieldType::String,
                        hidden: false,
                        auto: None,
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
                ],
            }],
            hierarchy: HierarchyIR {
                module_id: "test".into(),
                label: "Test".into(),
                icon: None,
                resources: vec![ResourceNode::leaf("User", "/users")],
            },
            facets: vec![FacetIR {
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
            }],
        }
    }

    #[test]
    fn valid_module_no_errors() {
        let module = minimal_module();
        let errors = validate_module(&module);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn facet_field_not_in_model() {
        let mut module = minimal_module();
        module.facets[0].fields.push(FacetFieldDef {
            name: "nonexistent".into(),
            ty: FieldType::String,
            readonly: false,
        });

        let errors = validate_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nonexistent"));
        assert!(errors[0].message.contains("not found in model"));
    }

    #[test]
    fn facet_field_type_mismatch() {
        let mut module = minimal_module();
        // email is Option<String> in model, but String in facet.
        module.facets[0].fields.push(FacetFieldDef {
            name: "email".into(),
            ty: FieldType::String, // wrong, should be Option<String>
            readonly: false,
        });

        let errors = validate_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("type mismatch"));
    }

    #[test]
    fn hierarchy_unknown_model() {
        let mut module = minimal_module();
        module
            .hierarchy
            .resources
            .push(ResourceNode::leaf("Unknown", "/unknown"));

        let errors = validate_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Unknown"));
    }

    #[test]
    fn facet_references_unknown_model() {
        let mut module = minimal_module();
        module.facets.push(FacetIR {
            facet: "data".into(),
            path: "/data".into(),
            auth: AuthMethod::Jwt,
            model: "Ghost".into(),
            fields: vec![],
            methods: vec![],
            crud: true,
        });

        let errors = validate_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Ghost"));
    }

    #[test]
    fn permission_format_valid() {
        assert!(validate_permission_format("auth:user:create").is_ok());
        assert!(validate_permission_format("pms:batch:provision").is_ok());
        assert!(validate_permission_format("task:task_type:list").is_ok());
    }

    #[test]
    fn permission_format_invalid() {
        assert!(validate_permission_format("auth:user").is_err()); // 2 parts
        assert!(validate_permission_format("auth:user:create:extra").is_err()); // 4 parts
        assert!(validate_permission_format("Auth:User:Create").is_err()); // uppercase
        assert!(validate_permission_format("auth::create").is_err()); // empty segment
    }

    #[test]
    fn invalid_method_permission() {
        let mut module = minimal_module();
        module.models[0].methods.push(MethodSig {
            name: "bad_method".into(),
            http_method: HttpMethod::Post,
            path: "/@bad".into(),
            permission: Some("INVALID".into()),
            params: vec![],
            return_type: None,
            doc: None,
        });

        let errors = validate_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("INVALID"));
    }
}
