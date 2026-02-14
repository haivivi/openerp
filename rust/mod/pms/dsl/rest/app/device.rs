//! AppDevice — what the mobile app sees for a device.
//! Subset of the full Device model. No secret, no internal fields.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDevice {
    pub sn: String,
    pub model: u32,
    pub status: String,
    pub display_name: Option<String>,
}
