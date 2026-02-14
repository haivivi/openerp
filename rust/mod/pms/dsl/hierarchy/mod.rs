use openerp_store::HierarchyNode;

/// PMS module resource hierarchy.
///
/// Data relationships:
/// - Model is the root entity: devices, batches, firmware belong to a model
/// - Batch produces Devices (via provisioning)
/// - LicenseImport contains Licenses (import batch → license entries)
/// - Device, Batch, License also top-level for direct access
pub fn hierarchy() -> Vec<HierarchyNode> {
    vec![
        HierarchyNode {
            resource: "model", label: "Models", icon: "cube",
            description: "Device model/series definitions",
            children: vec![
                HierarchyNode::leaf("device", "Devices", "desktop", "Devices of this model"),
                HierarchyNode::leaf("batch", "Batches", "package", "Production batches for this model"),
                HierarchyNode::leaf("firmware", "Firmware", "cpu", "Firmware versions for this model"),
            ],
        },
        HierarchyNode::leaf("device", "Devices", "desktop", "Produced devices with SN"),
        HierarchyNode {
            resource: "batch", label: "Batches", icon: "package",
            description: "Production batches",
            children: vec![
                HierarchyNode::leaf("device", "Devices", "desktop", "Devices produced in this batch"),
            ],
        },
        HierarchyNode::leaf("firmware", "Firmware", "cpu", "Firmware versions"),
        HierarchyNode::leaf("license", "Licenses", "key", "Licenses (MIIT, WiFi, etc.)"),
        HierarchyNode {
            resource: "license_import", label: "Imports", icon: "file-text",
            description: "License import batches",
            children: vec![
                HierarchyNode::leaf("license", "Licenses", "key", "Licenses from this import"),
            ],
        },
        HierarchyNode::leaf("segment", "Segments", "sliders", "SN encoding segments"),
    ]
}
