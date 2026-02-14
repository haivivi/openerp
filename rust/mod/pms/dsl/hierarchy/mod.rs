use openerp_store::HierarchyNode;

pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode::leaf("model", "Models", "cube", "Device model/series definitions"),
        HierarchyNode::leaf("device", "Devices", "desktop", "Produced devices with SN"),
        HierarchyNode::leaf("batch", "Batches", "package", "Production batches"),
        HierarchyNode::leaf("firmware", "Firmware", "cpu", "Firmware versions"),
        HierarchyNode::leaf("license", "Licenses", "key", "Licenses (MIIT, WiFi, etc.)"),
        HierarchyNode::leaf("license_import", "Imports", "file-text", "License import batches"),
        HierarchyNode::leaf("segment", "Segments", "sliders", "SN encoding segments"),
    ]
}
