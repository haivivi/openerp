use openerp_macro::model;
use openerp_types::*;

/// A tweet (post).
#[model(module = "twitter")]
pub struct Tweet {
    pub id: Id,
    pub author_id: Id,
    pub content: String,
    pub image_url: Option<Url>,
    pub like_count: u32,
    pub reply_count: u32,
    pub reply_to_id: Option<Id>,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
