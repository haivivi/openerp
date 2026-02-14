//! Auth UI widget overrides.
//!
//! Each file declares widget configurations for specific fields.
//! Fields not listed here use their type's default widget.

mod permission_picker;
mod textarea;
mod password;
mod url;

use openerp_store::WidgetOverride;

/// Collect all UI widget overrides for the auth module.
pub fn overrides() -> Vec<WidgetOverride> {
    let mut all = Vec::new();
    all.extend(permission_picker::overrides());
    all.extend(textarea::overrides());
    all.extend(password::overrides());
    all.extend(url::overrides());
    all
}
