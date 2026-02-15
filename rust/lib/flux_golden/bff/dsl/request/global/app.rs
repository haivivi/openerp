//! App lifecycle requests.

use flux_derive::request;

/// Initialize app state.
#[request("app/initialize")]
pub struct InitializeReq;

/// Refresh timeline.
#[request("timeline/load")]
pub struct TimelineLoadReq;

/// Update a compose form field.
#[request("compose/update-field")]
pub struct ComposeUpdateReq {
    pub field: String,
    pub value: String,
}
