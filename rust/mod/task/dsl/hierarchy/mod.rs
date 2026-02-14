use openerp_store::HierarchyNode;

pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode::leaf("task", "Tasks", "pulse", "Async task instances"),
        HierarchyNode::leaf("task_type", "Task Types", "file-text", "Task type definitions"),
    ]
}
