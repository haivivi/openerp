use openerp_store::HierarchyNode;

/// PMS module resource hierarchy.
///
/// Each resource appears exactly once, at its logical owner.
pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode {
            resource: "model", label: "Models", icon: "cube",
            description: "Device model/series definitions",
            children: vec![
                HierarchyNode {
                    resource: "batch", label: "Batches", icon: "package",
                    description: "Production batches",
                    children: vec![
                        HierarchyNode::leaf("device", "Devices", "desktop", "Produced devices"),
                    ],
                },
                HierarchyNode::leaf("firmware", "Firmware", "cpu", "Firmware versions"),
            ],
        },
        HierarchyNode {
            resource: "license_import", label: "License Imports", icon: "file-text",
            description: "License import batches",
            children: vec![
                HierarchyNode::leaf("license", "Licenses", "key", "License entries"),
            ],
        },
        HierarchyNode::leaf("segment", "Segments", "sliders", "SN encoding segments"),
    ]
}
