use openerp_store::{widget, WidgetOverride};

pub fn overrides() -> Vec<WidgetOverride> {
    vec![
        widget!(textarea {
            rows: 3,
            placeholder: "Brief description"
        } => [
            Role.description,
            Group.description,
        ]),
    ]
}
