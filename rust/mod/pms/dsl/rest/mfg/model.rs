//! MfgModel — product model info for MFG app.
//! Factory needs: code, series name, display name.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MfgModel {
    pub code: u32,
    pub series_name: String,
    pub display_name: Option<String>,
}
