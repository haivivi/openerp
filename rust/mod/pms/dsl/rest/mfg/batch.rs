//! MfgBatch — production batch view for MFG app.
//! Factory needs: batch info, model, quantity, provisioning progress.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MfgBatch {
    pub id: String,
    pub model: u32,
    pub quantity: u32,
    pub provisioned_count: u32,
    pub status: String,
    pub display_name: Option<String>,
}
