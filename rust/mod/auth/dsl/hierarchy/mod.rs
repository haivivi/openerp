use openerp_store::HierarchyNode;

/// Auth module resource hierarchy.
///
/// Each resource appears exactly once, at its logical owner.
pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode {
            resource: "user", label: "Users", icon: "users",
            description: "User identity and account management",
            children: vec![
                HierarchyNode::leaf("session", "Sessions", "clock", "Login sessions"),
                HierarchyNode::leaf("policy", "Policies", "lock", "Access policies"),
            ],
        },
        HierarchyNode::leaf("role", "Roles", "shield", "Permission roles for access control"),
        HierarchyNode {
            resource: "group", label: "Groups", icon: "stack",
            description: "Organizational groups and hierarchy",
            children: vec![
                HierarchyNode::leaf("group", "Sub-groups", "stack", "Child groups"),
            ],
        },
        HierarchyNode::leaf("provider", "Providers", "globe", "OAuth provider configuration"),
    ]
}
