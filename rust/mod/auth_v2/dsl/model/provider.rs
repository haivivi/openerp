use openerp_macro::model;
use openerp_types::*;

#[model(module = "auth")]
pub struct Provider {
    pub id: Id,
    pub name: String,
    pub provider_type: String,
    pub client_id: String,
    pub client_secret: Option<Secret>,
    pub auth_url: Url,
    pub token_url: Url,
    pub userinfo_url: Option<Url>,
    pub scopes: Vec<String>,
    pub redirect_url: Url,
    pub enabled: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
