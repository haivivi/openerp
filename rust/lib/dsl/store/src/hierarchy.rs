//! Resource hierarchy — tree structure for navigation and relations.

use serde_json::{json, Value};

/// A node in the resource hierarchy tree.
#[derive(Debug, Clone)]
pub struct HierarchyNode {
    /// Resource name (snake_case, matches ResourceDef.name). e.g. "model", "device"
    pub resource: &'static str,
    /// Display label. e.g. "Models", "Devices"
    pub label: &'static str,
    /// Icon name (Phosphor). e.g. "cube", "monitor"
    pub icon: &'static str,
    /// Short description.
    pub description: &'static str,
    /// Child resources (shown nested in sidebar, as tabs in detail page).
    pub children: Vec<HierarchyNode>,
}

impl HierarchyNode {
    /// Create a leaf node (no children).
    pub fn leaf(
        resource: &'static str,
        label: &'static str,
        icon: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            resource,
            label,
            icon,
            description,
            children: vec![],
        }
    }

    /// Convert to JSON for schema output.
    pub fn to_json(&self) -> Value {
        json!({
            "resource": self.resource,
            "label": self.label,
            "icon": self.icon,
            "description": self.description,
            "children": self.children.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
        })
    }
}
