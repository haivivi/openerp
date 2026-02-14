use openerp_macro::model;
use openerp_types::*;

/// A produced device, created by Batch provisioning.
#[model(module = "pms")]
pub struct Device {
    pub sn: String,
    pub secret: Secret,
    pub model: u32,
    pub status: String,
    pub sku: Option<String>,
    pub imei: Vec<String>,
    pub licenses: Vec<String>,
    // display_name, description, metadata, created_at, updated_at → auto
}
