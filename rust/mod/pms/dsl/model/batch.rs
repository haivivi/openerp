use openerp_macro::model;
use openerp_types::*;

use super::status::BatchStatus;

/// A production batch. Provisioning generates Devices.
#[model(module = "pms")]
pub struct Batch {
    pub id: Id,
    pub model: u32,
    pub quantity: u32,
    pub provisioned_count: u32,
    pub status: BatchStatus,
    // display_name, description, metadata, created_at, updated_at → auto
}
