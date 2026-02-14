use oe_macro::model;
use oe_types::*;

/// A license import/generation batch record.
#[model(module = "pms")]
pub struct LicenseImport {
    pub id: Id,
    pub license_type: String,
    pub source: String,
    pub count: u64,
    pub allocated_count: u64,
    // display_name, description, metadata, created_at, updated_at → auto
}
