use openerp_store::HierarchyNode;

/// Auth module resource hierarchy.
///
/// Data relationships:
/// - User owns Sessions (login records) and Policies (access grants)
/// - Group contains sub-Groups (tree) and Policies
/// - Role, Policy, Session, Provider are also top-level for direct access
pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode {
            resource: "user", label: "Users", icon: "users",
            description: "User identity and account management",
            children: vec![
                HierarchyNode::leaf("session", "Sessions", "clock", "User's login sessions"),
                HierarchyNode::leaf("policy", "Policies", "lock", "User's access policies"),
            ],
        },
        HierarchyNode::leaf("role", "Roles", "shield", "Permission roles for access control"),
        HierarchyNode {
            resource: "group", label: "Groups", icon: "stack",
            description: "Organizational groups and hierarchy",
            children: vec![
                HierarchyNode::leaf("group", "Sub-groups", "stack", "Child groups"),
                HierarchyNode::leaf("policy", "Policies", "lock", "Group access policies"),
            ],
        },
        HierarchyNode::leaf("policy", "Policies", "lock", "Access control policies (who-what-how)"),
        HierarchyNode::leaf("session", "Sessions", "clock", "Login sessions and JWT tracking"),
        HierarchyNode::leaf("provider", "Providers", "globe", "OAuth provider configuration"),
    ]
}
