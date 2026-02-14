use oe_store::{widget, WidgetOverride};

pub fn overrides() -> Vec<WidgetOverride> {
    vec![
        widget!(permission_picker {
            source: "schema.permissions",
            layout: "modal"
        } => [Role.permissions]),
    ]
}
