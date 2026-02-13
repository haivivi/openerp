//! Parser for `#[module]` hierarchy definitions.
//!
//! Reads a module with nested resource declarations and produces `HierarchyIR`.
//!
//! Since Rust doesn't support custom `resource` keyword syntax natively,
//! we use a function-call-based DSL that the macro processes:
//!
//! ```ignore
//! #[module(id = "pms", label = "Product Management", icon = "box")]
//! pub fn pms_hierarchy() -> HierarchyDef {
//!     hierarchy! {
//!         resource("Model", "/models") {
//!             resource("Device", "/devices");
//!             resource("Batch", "/batches");
//!             resource("Firmware", "/firmware");
//!         }
//!         resource("Device", "/devices");
//!         resource("Batch", "/batches") {
//!             resource("Device", "/devices");
//!         }
//!         resource("License", "/licenses");
//!     }
//! }
//! ```
//!
//! For the parser, we take a simpler approach: parse from a structured
//! Rust data literal that can be used at compile time.

use openerp_ir::{HierarchyIR, ResourceNode};

/// Build a HierarchyIR programmatically (used by the macro expansion).
/// This is the "parse" step for hierarchy — the macro will call this
/// with data extracted from the DSL syntax.
pub fn build_hierarchy(
    module_id: String,
    label: String,
    icon: Option<String>,
    resources: Vec<ResourceNode>,
) -> HierarchyIR {
    HierarchyIR {
        module_id,
        label,
        icon,
        resources,
    }
}

/// Helper to build a ResourceNode tree from a simple nested structure.
/// Used by the `hierarchy!` macro expansion.
pub fn resource_node(
    model: impl Into<String>,
    path: impl Into<String>,
    children: Vec<ResourceNode>,
) -> ResourceNode {
    ResourceNode {
        model: model.into(),
        path: path.into(),
        label: None,
        icon: None,
        children,
    }
}

/// Helper with label and icon.
pub fn resource_node_full(
    model: impl Into<String>,
    path: impl Into<String>,
    label: Option<String>,
    icon: Option<String>,
    children: Vec<ResourceNode>,
) -> ResourceNode {
    ResourceNode {
        model: model.into(),
        path: path.into(),
        label,
        icon,
        children,
    }
}

/// Parse `#[module(...)]` attributes from a syn Attribute.
pub fn parse_module_attrs(
    attr: &syn::Attribute,
) -> syn::Result<(String, String, Option<String>)> {
    let mut id = String::new();
    let mut label = String::new();
    let mut icon = None;

    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            let key = ident.to_string();
            match key.as_str() {
                "id" => {
                    let v = meta.value()?;
                    let lit: syn::Lit = v.parse()?;
                    if let syn::Lit::Str(s) = lit {
                        id = s.value();
                    }
                }
                "label" => {
                    let v = meta.value()?;
                    let lit: syn::Lit = v.parse()?;
                    if let syn::Lit::Str(s) = lit {
                        label = s.value();
                    }
                }
                "icon" => {
                    let v = meta.value()?;
                    let lit: syn::Lit = v.parse()?;
                    if let syn::Lit::Str(s) = lit {
                        icon = Some(s.value());
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })?;

    if id.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "module requires id: #[module(id = \"...\")]",
        ));
    }
    if label.is_empty() {
        label = id.clone();
    }

    Ok((id, label, icon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_pms_hierarchy() {
        let h = build_hierarchy(
            "pms".into(),
            "Product Management".into(),
            Some("box".into()),
            vec![
                resource_node(
                    "Model",
                    "/models",
                    vec![
                        resource_node("Device", "/devices", vec![]),
                        resource_node("Batch", "/batches", vec![]),
                        resource_node("Firmware", "/firmware", vec![]),
                    ],
                ),
                resource_node("Device", "/devices", vec![]),
                resource_node(
                    "Batch",
                    "/batches",
                    vec![resource_node("Device", "/devices", vec![])],
                ),
                resource_node("License", "/licenses", vec![]),
                resource_node("LicenseImport", "/license-imports", vec![]),
                resource_node("Segment", "/segments", vec![]),
            ],
        );

        assert_eq!(h.module_id, "pms");
        assert_eq!(h.top_level_models().len(), 6);

        let routes = h.resolve_routes("/pms");
        // Should have: Model, Model>Device, Model>Batch, Model>Firmware,
        //              Device, Batch, Batch>Device, License, LicenseImport, Segment
        assert!(routes.len() >= 10);

        // Check nested route
        assert!(routes
            .iter()
            .any(|r| r.model == "Device" && r.parents == vec!["Model"]));
    }

    #[test]
    fn build_auth_hierarchy() {
        let h = build_hierarchy(
            "auth".into(),
            "Authentication".into(),
            Some("shield".into()),
            vec![
                resource_node(
                    "User",
                    "/users",
                    vec![
                        resource_node("Session", "/sessions", vec![]),
                        resource_node("Policy", "/policies", vec![]),
                    ],
                ),
                resource_node("Role", "/roles", vec![]),
                resource_node(
                    "Group",
                    "/groups",
                    vec![
                        resource_node("Group", "/children", vec![]),
                        resource_node("Policy", "/policies", vec![]),
                    ],
                ),
                resource_node("Policy", "/policies", vec![]),
                resource_node("Session", "/sessions", vec![]),
                resource_node("Provider", "/providers", vec![]),
            ],
        );

        assert_eq!(h.module_id, "auth");
        assert_eq!(h.top_level_models().len(), 6);
    }
}
