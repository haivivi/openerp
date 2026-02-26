use openerp_macro::model;
use openerp_types::*;

use super::User;

/// In-app message (站内信).
///
/// Uses `LocalizedText` for multi-language content.
/// The facet handler selects the user's language and returns
/// the appropriate translation, falling back to English.
#[model(module = "twitter", name = "twitter/messages/{id}")]
pub struct Message {
    pub id: Id,
    /// "system" | "broadcast" | "personal"
    pub kind: String,
    pub sender: Option<Name<User>>,
    pub recipient: Option<Name<User>>,
    pub title: LocalizedText,
    pub body: LocalizedText,
    pub read: bool,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
