//! OpenERP DSL Web Components.
//!
//! Reusable, schema-driven web UI components embedded as static strings.
//! Other projects depend on this crate and serve these via their HTTP server.
//!
//! Usage:
//! ```ignore
//! use oe_web;
//!
//! app.route("/", get(|| async { Html(oe_web::login_html()) }));
//! app.route("/dashboard", get(|| async { Html(oe_web::dashboard_html()) }));
//! ```
//!
//! The dashboard reads `/meta/schema` to dynamically render:
//! - Module navigation (top bar mega-menu)
//! - Resource sidebar (from hierarchy)
//! - Data tables (columns from model fields)
//! - Create/edit forms (widgets from field types + UI overrides)
//! - Permission picker (from schema.permissions)

/// Login page HTML.
/// Features: split layout (brand + form), shadcn dark theme, JWT storage.
pub fn login_html() -> &'static str {
    include_str!("components/login.html")
}

/// Dashboard page HTML.
/// Schema-driven: reads /meta/schema for all rendering.
/// Features: module mega-menu, resource sidebar, data tables,
/// create dialog, permission picker, Phosphor icons.
pub fn dashboard_html() -> &'static str {
    include_str!("components/dashboard.html")
}
