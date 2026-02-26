use openerp_macro::model;
use openerp_types::*;

use super::User;

/// A tweet (post).
#[model(module = "twitter", name = "twitter/tweets/{id}")]
pub struct Tweet {
    pub id: Id,
    pub author: Name<User>,
    pub content: String,
    pub image_url: Option<Url>,
    pub like_count: u32,
    pub reply_count: u32,
    pub reply_to: Option<Name<Tweet>>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
