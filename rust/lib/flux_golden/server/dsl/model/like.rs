use openerp_macro::model;
use openerp_types::*;

use super::{User, Tweet};

/// A like record (user liked a tweet). Composite key: `{user_id}:{tweet_id}`.
#[model(module = "twitter")]
pub struct Like {
    pub id: Id,
    pub user: Name<User>,
    pub tweet: Name<Tweet>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
