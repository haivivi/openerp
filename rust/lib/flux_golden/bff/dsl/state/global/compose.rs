//! Compose state — stored at `compose/state`.

use flux_derive::state;
use serde::{Deserialize, Serialize};

/// Tweet compose form state.
#[state("compose/state")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeState {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    pub busy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ComposeState {
    pub fn empty() -> Self {
        Self {
            content: String::new(),
            reply_to_id: None,
            busy: false,
            error: None,
        }
    }
}
