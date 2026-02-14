use oe_store::{widget, WidgetOverride};

pub fn overrides() -> Vec<WidgetOverride> {
    vec![
        widget!(password => [
            User.password_hash,
            Provider.client_secret,
        ]),
    ]
}
