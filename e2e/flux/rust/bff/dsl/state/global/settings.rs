//! Settings state — stored at `settings/state`.

use flux_derive::state;
use serde::{Deserialize, Serialize};

/// User settings / edit profile form.
#[state("settings/state")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsState {
    pub display_name: String,
    pub bio: String,
    pub busy: bool,
    pub saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Password change form state.
#[state("settings/password")]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordState {
    pub busy: bool,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
