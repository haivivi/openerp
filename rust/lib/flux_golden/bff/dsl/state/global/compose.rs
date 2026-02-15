//! Compose state — stored at `compose/state`.

use flux_derive::state;

/// Tweet compose form state.
#[state("compose/state")]
pub struct ComposeState {
    pub content: String,
    pub reply_to_id: Option<String>,
    pub busy: bool,
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
