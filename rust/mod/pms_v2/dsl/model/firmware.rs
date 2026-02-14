use openerp_macro::model;
use openerp_types::*;

/// Device firmware version. Compound key: model + semver.
#[model(module = "pms")]
pub struct Firmware {
    pub id: Id,
    pub model: u32,
    pub semver: SemVer,
    pub build: u64,
    pub status: String,
    pub release_notes: Option<String>,
    // display_name, description, metadata, created_at, updated_at → auto
}
