use openerp_store::HierarchyNode;

pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode {
            resource: "model", label: "Models", icon: "cube",
            description: "Device model/series definitions",
            children: vec![
                HierarchyNode::leaf("device", "Devices", "desktop", "Devices of this model"),
                HierarchyNode::leaf("batch", "Batches", "package", "Production batches"),
                HierarchyNode::leaf("firmware", "Firmware", "cpu", "Firmware versions"),
            ],
        },
        HierarchyNode::leaf("device", "Devices", "desktop", "Produced devices with SN"),
        HierarchyNode {
            resource: "batch", label: "Batches", icon: "package",
            description: "Production batches",
            children: vec![
                HierarchyNode::leaf("device", "Devices", "desktop", "Devices in this batch"),
            ],
        },
        HierarchyNode::leaf("firmware", "Firmware", "cpu", "Firmware versions"),
        HierarchyNode::leaf("license", "Licenses", "key", "Licenses (MIIT, WiFi, etc.)"),
        HierarchyNode {
            resource: "license_import", label: "Imports", icon: "file-text",
            description: "License import batches",
            children: vec![
                HierarchyNode::leaf("license", "Licenses", "key", "Imported licenses"),
            ],
        },
        HierarchyNode::leaf("segment", "Segments", "sliders", "SN encoding segments"),
    ]
}
