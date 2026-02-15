//! App lifecycle requests.

/// Initialize app state.
// #[request("app/initialize")]
#[derive(Debug, Clone)]
pub struct InitializeReq;

impl InitializeReq {
    pub const PATH: &'static str = "app/initialize";
}

/// Refresh timeline.
// #[request("timeline/load")]
#[derive(Debug, Clone)]
pub struct TimelineLoadReq;

impl TimelineLoadReq {
    pub const PATH: &'static str = "timeline/load";
}

/// Update a compose form field.
// #[request("compose/update-field")]
#[derive(Debug, Clone)]
pub struct ComposeUpdateReq {
    pub field: String,
    pub value: String,
}

impl ComposeUpdateReq {
    pub const PATH: &'static str = "compose/update-field";
}
