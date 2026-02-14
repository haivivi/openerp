//! MfgFirmware — firmware view for MFG app.
//! Factory needs: version info + status for flashing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MfgFirmware {
    pub id: String,
    pub model: u32,
    pub semver: String,
    pub build: u64,
    pub status: String,
    pub display_name: Option<String>,
}
