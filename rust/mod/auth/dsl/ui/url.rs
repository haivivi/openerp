use openerp_store::{widget, WidgetOverride};

pub fn overrides() -> Vec<WidgetOverride> {
    vec![
        widget!(url {
            placeholder: "https://provider.com/oauth/authorize"
        } => [
            Provider.auth_url,
            Provider.token_url,
            Provider.userinfo_url,
            Provider.redirect_url,
        ]),
    ]
}
