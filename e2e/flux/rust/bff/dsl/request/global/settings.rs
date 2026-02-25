//! Settings requests.

use flux_derive::request;

/// Load current user's profile into settings form.
#[request("settings/load")]
pub struct SettingsLoadReq;

/// Save profile changes (display name, bio).
#[request("settings/save")]
pub struct SettingsSaveReq {
    pub display_name: String,
    pub bio: String,
}

/// Change password.
#[request("settings/change-password")]
pub struct ChangePasswordReq {
    pub old_password: String,
    pub new_password: String,
}
