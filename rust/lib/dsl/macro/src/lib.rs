//! OpenERP DSL proc macros.
//!
//! `#[model(module = "auth")]` — defines a model struct:
//!   - Adds Serialize/Deserialize with camelCase
//!   - Generates Field consts for each field (Self::id, Self::email, ...)
//!   - Embeds IR metadata as __DSL_IR const
//!   - Generates __DSL_FIELDS const with all Field descriptors

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod enum_impl;
mod facet;
mod flatbuf;
mod handler;
mod model;
mod util;

/// Define a DSL model.
///
/// ```ignore
/// #[model(module = "auth")]
/// pub struct User {
///     pub id: Id,
///     pub name: String,
///     pub email: Option<Email>,
///     pub active: bool,
/// }
/// ```
///
/// Generates:
/// - `#[derive(Debug, Clone, Serialize, Deserialize)]` with `#[serde(rename_all = "camelCase")]`
/// - Field consts: `User::id`, `User::email`, etc. (type `openerp_types::Field`)
/// - `User::__DSL_IR` — JSON metadata for codegen/schema
/// - `User::__DSL_FIELDS` — array of all Field descriptors
/// - `User::__DSL_MODULE` — module name string
#[proc_macro_attribute]
pub fn model(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    model::expand(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

/// Define a DSL enum — a first-class status/category type.
///
/// ```ignore
/// #[dsl_enum(module = "pms")]
/// pub enum BatchStatus {
///     Draft,
///     InProgress,
///     Completed,
///     Cancelled,
/// }
/// ```
///
/// Generates:
/// - `#[derive(Serialize, Deserialize)]` with `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`
/// - `Display` / `FromStr` (SCREAMING_SNAKE_CASE, case-insensitive parse)
/// - `Default` (first variant)
/// - `DslEnum` trait impl with `variants()` for schema/UI
/// - `__dsl_ir()` returning enum metadata JSON
#[proc_macro_attribute]
pub fn dsl_enum(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemEnum);
    enum_impl::expand(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

/// Define a facet — a typed API surface for a specific consumer.
///
/// ```ignore
/// #[facet(name = "mfg", module = "pms")]
/// pub mod mfg {
///     #[resource(path = "/models", pk = "code")]
///     pub struct MfgModel {
///         pub code: u32,
///         pub series_name: String,
///     }
///
///     #[action(method = "POST", path = "/batches/{id}/@provision")]
///     pub type Provision = fn(id: String, req: ProvisionRequest) -> ProvisionResponse;
/// }
/// ```
///
/// Generates:
/// - Serde-derived structs for each `#[resource]`
/// - Facet metadata (`__FACET_NAME`, `__FACET_MODULE`, `__facet_ir()`)
/// - Typed HTTP client struct (`MfgClient`) with methods for each resource and action
///
/// Handlers are **not** generated — they remain hand-written.
/// Use [`impl_handler!`] to bind handlers to actions for compile-time completeness checks.
#[proc_macro_attribute]
pub fn facet(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemMod);
    facet::expand(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

/// Bind a handler route function to an `#[action]` declaration.
///
/// Generates a trait impl on the facet's `__Handlers` registry.
/// When the module calls `action_router()`, all actions must have a
/// corresponding `impl_handler!` — otherwise compilation fails.
///
/// ```ignore
/// // In facet definition:
/// #[action(method = "POST", path = "/batches/{id}/@provision")]
/// pub type Provision = fn(id: String, req: ProvisionRequest) -> ProvisionResponse;
///
/// // In handler module:
/// openerp_macro::impl_handler!(mfg::Provision, provision::routes);
///
/// fn routes(kv: Arc<dyn KVStore>) -> Router { /* ... */ }
/// ```
#[proc_macro]
pub fn impl_handler(input: TokenStream) -> TokenStream {
    handler::expand(input.into())
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}
