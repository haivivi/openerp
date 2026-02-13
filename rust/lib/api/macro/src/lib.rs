//! OpenERP DSL Proc Macros
//!
//! Attribute macros for the five-layer DSL:
//!
//! - `#[model]`      — defines a model struct, emits with Serialize/Deserialize
//! - `#[persistent]` — generates CRUD service methods for a model's DB struct
//! - `#[facet]`      — generates Axum router for a REST API surface
//! - `#[module_def]` — (reserved) module hierarchy definition

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod gen_model;
mod gen_persistent;
mod gen_facet;

/// Attribute macro for model definitions.
///
/// Usage:
/// ```ignore
/// #[model(module = "auth")]
/// #[key(id)]
/// pub struct User {
///     pub id: String,
///     pub name: String,
///     pub email: Option<String>,
/// }
/// ```
///
/// Generates:
/// - Re-emits the struct with `#[derive(Debug, Clone, Serialize, Deserialize)]`
/// - Embeds IR metadata as a const for use by other macros and codegen
#[proc_macro_attribute]
pub fn model(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    gen_model::expand_model(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

/// Attribute macro for persistent (DB) definitions.
///
/// Usage:
/// ```ignore
/// #[persistent(User, store = "kv")]
/// #[key(id)]
/// #[unique(email)]
/// #[index(name)]
/// pub struct UserDB {
///     #[auto(uuid)]
///     pub id: String,
///     pub name: String,
///     pub password_hash: String,
///     #[auto(create_timestamp)]
///     pub created_at: String,
/// }
/// ```
///
/// Generates:
/// - Re-emits the DB struct with Serialize/Deserialize
/// - A service struct with CRUD methods: get, list, create, update, delete
/// - Key serialization/deserialization helpers
#[proc_macro_attribute]
pub fn persistent(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    gen_persistent::expand_persistent(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

/// Attribute macro for facet (REST API surface) definitions.
///
/// Usage:
/// ```ignore
/// #[facet(path = "/data", auth = "jwt", model = "User")]
/// pub struct DataUser {
///     pub id: String,
///     pub name: String,
/// }
/// ```
///
/// Generates:
/// - An Axum Router with CRUD routes
/// - Permission checks via Authenticator trait
#[proc_macro_attribute]
pub fn facet(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    gen_facet::expand_facet(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}
