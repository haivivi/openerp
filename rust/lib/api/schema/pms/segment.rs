//! Segment resource — SN segment dimension entries.
//!
//! Pure db_resource: standard CRUD.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

// #[db_resource(module = "pms", table = "segments", display_name = "Segment")]
// #[permission(create = "pms:segment:create")]
// #[permission(read = "pms:segment:read")]
// #[permission(update = "pms:segment:update")]
// #[permission(delete = "pms:segment:delete")]
// #[permission(list = "pms:segment:list")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SNSegment {
    /// Dimension name (e.g. "manufacturer", "channel").
    pub dimension: String,

    /// Numeric code for this entry.
    pub code: u32,

    /// Human-readable name (e.g. "Foxconn", "Tmall").
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
