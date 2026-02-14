use oe_macro::model;
use oe_types::*;

/// SN segment dimension entry (e.g. manufacturer, channel).
#[model(module = "pms")]
pub struct Segment {
    pub dimension: String,
    pub code: u32,
    // display_name, description, metadata, created_at, updated_at → auto
}
