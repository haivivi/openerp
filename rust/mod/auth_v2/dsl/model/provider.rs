use oe_macro::model;
use oe_types::*;

#[model(module = "auth")]
pub struct Provider {
    pub id: Id,
    pub provider_type: String,
    pub client_id: String,
    pub client_secret: Option<Secret>,
    pub auth_url: Url,
    pub token_url: Url,
    pub userinfo_url: Option<Url>,
    pub scopes: Vec<String>,
    pub redirect_url: Url,
    pub enabled: bool,
    // display_name, description, metadata, created_at, updated_at → auto-injected
}
