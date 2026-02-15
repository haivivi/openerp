//! Flux proc macros for cross-platform state engine.
//!
//! - `#[state("path")]` — mark a struct as a Flux state type
//! - `#[request("path")]` — mark a struct as a Flux request type
//!
//! Both generate:
//! - `impl StructName { pub const PATH: &'static str = "the/path"; }`
//! - Automatically add `#[derive(Debug, Clone)]`
//!
//! `#[state]` additionally adds `PartialEq` derive (states need comparison).

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod state;
mod request;

/// Define a Flux state type.
///
/// ```ignore
/// #[state("auth/state")]
/// pub struct AuthState {
///     pub phase: AuthPhase,
///     pub busy: bool,
/// }
/// ```
///
/// Generates:
/// - `#[derive(Debug, Clone, PartialEq)]` (if not already present)
/// - `impl AuthState { pub const PATH: &'static str = "auth/state"; }`
#[proc_macro_attribute]
pub fn state(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    state::expand(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}

/// Define a Flux request type.
///
/// ```ignore
/// #[request("auth/login")]
/// pub struct LoginReq {
///     pub username: String,
/// }
/// ```
///
/// Generates:
/// - `#[derive(Debug, Clone)]` (if not already present)
/// - `impl LoginReq { pub const PATH: &'static str = "auth/login"; }`
#[proc_macro_attribute]
pub fn request(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::ItemStruct);
    request::expand(attr.into(), item)
        .unwrap_or_else(|e| e.to_compile_error().into())
        .into()
}
