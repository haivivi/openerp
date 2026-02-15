//! MfgDevice — device view for MFG app.
//! Factory operators see: SN, model, status, SKU, IMEI list.
//! No secret exposed. Licenses shown for verification.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MfgDevice {
    pub sn: String,
    pub model: u32,
    pub status: String,
    pub sku: Option<String>,
    pub imei: Vec<String>,
    pub licenses: Vec<String>,
    pub display_name: Option<String>,
}
