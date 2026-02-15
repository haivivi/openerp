use openerp_macro::model;
use openerp_types::*;

/// A follow relationship. Composite key: `{follower_id}:{followee_id}`.
#[model(module = "twitter")]
pub struct Follow {
    pub id: Id,
    pub follower_id: Id,
    pub followee_id: Id,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
