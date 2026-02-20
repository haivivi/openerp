use openerp_macro::model;
use openerp_types::*;

/// A Twitter user account.
#[model(module = "twitter", name = "twitter/users/{id}")]
pub struct User {
    pub id: Id,
    pub username: String,
    pub password_hash: Option<PasswordHash>,
    pub bio: Option<String>,
    pub avatar: Option<Avatar>,
    pub follower_count: u32,
    pub following_count: u32,
    pub tweet_count: u32,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
