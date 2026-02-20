use openerp_macro::model;
use openerp_types::*;

use super::User;

/// A follow relationship. Composite key: `{follower}:{followee}`.
#[model(module = "twitter")]
pub struct Follow {
    pub id: Id,
    pub follower: Name<User>,
    pub followee: Name<User>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
