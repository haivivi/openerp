//! OpenERP DSL proc macros.
//!
//! `#[model(module = "auth")]` — defines a model struct:
//!   - Adds Serialize/Deserialize with camelCase
//!   - Generates Field consts for each field (Self::id, Self::email, ...)
//!   - Embeds IR metadata as __DSL_IR const
//!   - Generates __DSL_FIELDS const with all Field descriptors

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod facet;
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
#[proc_macro_attribute]
pub fn facet(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemMod);
    facet::expand(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}
