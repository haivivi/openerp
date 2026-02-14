use openerp_store::HierarchyNode;

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
                HierarchyNode::leaf("policy", "Policies", "lock", "Group policies"),
            ],
        },
        HierarchyNode::leaf("policy", "Policies", "lock", "Access control policies (who-what-how)"),
        HierarchyNode::leaf("session", "Sessions", "clock", "Login sessions and JWT tracking"),
        HierarchyNode::leaf("provider", "Providers", "globe", "OAuth provider configuration"),
    ]
}
