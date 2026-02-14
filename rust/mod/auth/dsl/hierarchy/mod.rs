use openerp_store::HierarchyNode;

pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode::leaf("user", "Users", "users", "User identity and account management"),
        HierarchyNode::leaf("role", "Roles", "shield", "Permission roles for access control"),
        HierarchyNode::leaf("group", "Groups", "stack", "Organizational groups and hierarchy"),
        HierarchyNode::leaf("policy", "Policies", "lock", "Access control policies (who-what-how)"),
        HierarchyNode::leaf("session", "Sessions", "clock", "Login sessions and JWT tracking"),
        HierarchyNode::leaf("provider", "Providers", "globe", "OAuth provider configuration"),
    ]
}
