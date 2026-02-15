use openerp_macro::model;
use openerp_types::*;

/// A like record (user liked a tweet). Composite key: `{user_id}:{tweet_id}`.
#[model(module = "twitter")]
pub struct Like {
    pub id: Id,
    pub user_id: Id,
    pub tweet_id: Id,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
