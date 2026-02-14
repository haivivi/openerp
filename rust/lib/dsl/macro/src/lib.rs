//! OpenERP DSL proc macros.
//!
//! `#[model(module = "auth")]` — defines a model struct:
//!   - Adds Serialize/Deserialize with camelCase
//!   - Generates Field consts for each field (Self::id, Self::email, ...)
//!   - Embeds IR metadata as __DSL_IR const
//!   - Generates __DSL_FIELDS const with all Field descriptors

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod model;

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
/// - Field consts: `User::id`, `User::email`, etc. (type `oe_types::Field`)
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
