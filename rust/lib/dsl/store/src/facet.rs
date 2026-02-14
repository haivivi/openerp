//! Facet support — multi-consumer API surfaces.
//!
//! A facet is an independent API surface for a specific consumer
//! (e.g. mobile app, firmware, agent). Each facet has:
//! - A name (just a string: "app", "gear", "app-next")
//! - A module it belongs to ("pms", "auth")
//! - A hand-built Axum Router with custom handlers
//!
//! The framework mounts facets at `/{facet_name}/{module}/...`.
//! Handlers are 100% hand-written. Framework only provides plumbing.

use axum::Router;

/// A facet definition returned by a module's `facet_routers()`.
pub struct FacetDef {
    /// Facet name — becomes the URL prefix. e.g. "app", "gear", "app-next"
    pub name: &'static str,
    /// Module this facet belongs to. e.g. "pms", "auth"
    pub module: &'static str,
    /// Hand-built Axum router with all the facet's routes.
    /// Routes inside should be relative (e.g. "/devices", "/devices/{id}").
    pub router: Router,
}
