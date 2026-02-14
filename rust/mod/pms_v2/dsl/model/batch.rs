use oe_macro::model;
use oe_types::*;

/// A production batch. Provisioning generates Devices.
#[model(module = "pms")]
pub struct Batch {
    pub id: Id,
    pub model: u32,
    pub quantity: u32,
    pub provisioned_count: u32,
    pub status: String,
    // display_name, description, metadata, created_at, updated_at → auto
}
