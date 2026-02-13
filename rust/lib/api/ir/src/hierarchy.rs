//! Hierarchy IR — resource nesting (routes + parent-child).
//!
//! Corresponds to `dsl/hierarchy/mod.rs`.
//! The nesting defines:
//! - URL route structure
//! - Parent-child relationships (detail page tabs)
//! - Navigation structure (sidebar)

use serde::{Deserialize, Serialize};

/// A node in the resource nesting tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceNode {
    /// Model name (e.g. `User`, `Device`).
    pub model: String,

    /// URL path segment (e.g. `/users`, `/devices`).
    pub path: String,

    /// Display label for navigation (defaults to model name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Icon name for navigation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Child resources nested under this one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ResourceNode>,
}

impl ResourceNode {
    /// Create a leaf node (no children).
    pub fn leaf(model: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            path: path.into(),
            label: None,
            icon: None,
            children: vec![],
        }
    }

    /// Recursively find all unique model names in this subtree.
    pub fn all_models(&self) -> Vec<&str> {
        let mut result = vec![self.model.as_str()];
        for child in &self.children {
            result.extend(child.all_models());
        }
        result
    }
}

/// Complete hierarchy IR for a module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HierarchyIR {
    /// Module ID (e.g. `auth`, `pms`).
    pub module_id: String,

    /// Module display label (e.g. `Authentication`, `Product Management`).
    pub label: String,

    /// Module icon name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Top-level resources in the sidebar.
    pub resources: Vec<ResourceNode>,
}

impl HierarchyIR {
    /// Collect all top-level resource model names (for sidebar).
    pub fn top_level_models(&self) -> Vec<&str> {
        self.resources.iter().map(|r| r.model.as_str()).collect()
    }

    /// Resolve a full route path for a nested resource.
    /// E.g. for `Model > Device`, given module prefix `/pms`:
    ///   `/pms/models/:code/devices`
    pub fn resolve_routes(&self, prefix: &str) -> Vec<ResolvedRoute> {
        let mut routes = Vec::new();
        for resource in &self.resources {
            Self::resolve_node(&mut routes, prefix, resource, &[]);
        }
        routes
    }

    fn resolve_node(
        routes: &mut Vec<ResolvedRoute>,
        prefix: &str,
        node: &ResourceNode,
        parents: &[&ResourceNode],
    ) {
        // Build path: prefix + parent segments + this segment
        let mut path = prefix.to_string();
        for parent in parents {
            path.push_str(&parent.path);
            path.push_str("/:id"); // placeholder, real key comes from model
        }
        path.push_str(&node.path);

        routes.push(ResolvedRoute {
            model: node.model.clone(),
            path: path.clone(),
            parents: parents.iter().map(|p| p.model.clone()).collect(),
        });

        // Recurse into children
        let mut new_parents: Vec<&ResourceNode> = parents.to_vec();
        new_parents.push(node);
        for child in &node.children {
            Self::resolve_node(routes, prefix, child, &new_parents);
        }
    }
}

/// A resolved route with its full path and parent chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedRoute {
    pub model: String,
    pub path: String,
    pub parents: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pms_hierarchy() -> HierarchyIR {
        HierarchyIR {
            module_id: "pms".into(),
            label: "Product Management".into(),
            icon: Some("box".into()),
            resources: vec![
                ResourceNode {
                    model: "Model".into(),
                    path: "/models".into(),
                    label: Some("Models".into()),
                    icon: Some("box".into()),
                    children: vec![
                        ResourceNode::leaf("Device", "/devices"),
                        ResourceNode::leaf("Batch", "/batches"),
                        ResourceNode::leaf("Firmware", "/firmware"),
                    ],
                },
                ResourceNode::leaf("Device", "/devices"),
                ResourceNode {
                    model: "Batch".into(),
                    path: "/batches".into(),
                    label: None,
                    icon: None,
                    children: vec![ResourceNode::leaf("Device", "/devices")],
                },
            ],
        }
    }

    #[test]
    fn top_level_models() {
        let h = sample_pms_hierarchy();
        assert_eq!(h.top_level_models(), vec!["Model", "Device", "Batch"]);
    }

    #[test]
    fn resolve_routes() {
        let h = sample_pms_hierarchy();
        let routes = h.resolve_routes("/pms");

        // Top-level Model
        assert!(routes.iter().any(|r| r.path == "/pms/models" && r.parents.is_empty()));
        // Nested Device under Model
        assert!(routes
            .iter()
            .any(|r| r.path == "/pms/models/:id/devices" && r.parents == vec!["Model"]));
        // Top-level Device
        assert!(routes.iter().any(|r| r.path == "/pms/devices" && r.parents.is_empty()));
        // Device under Batch
        assert!(routes
            .iter()
            .any(|r| r.path == "/pms/batches/:id/devices" && r.parents == vec!["Batch"]));
    }

    #[test]
    fn all_models_recursive() {
        let h = sample_pms_hierarchy();
        let all = h.resources[0].all_models();
        assert!(all.contains(&"Model"));
        assert!(all.contains(&"Device"));
        assert!(all.contains(&"Batch"));
        assert!(all.contains(&"Firmware"));
    }
}
