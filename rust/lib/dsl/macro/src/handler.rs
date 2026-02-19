//! `impl_handler!` macro — binds a route function to an `#[action]` handler trait.
//!
//! Usage:
//! ```ignore
//! openerp_macro::impl_handler!(mfg::Provision, handlers::provision::routes);
//! ```
//!
//! Expands to a trait impl on the facet's `__Handlers` registry:
//! ```ignore
//! impl mfg::__ProvisionHandler for mfg::__Handlers {
//!     fn route(kv: Arc<dyn KVStore>) -> Router {
//!         handlers::provision::routes(kv)
//!     }
//! }
//! ```

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Path, Token};
use syn::parse::{Parse, ParseStream};

struct Input {
    action_path: Path,
    _comma: Token![,],
    handler_fn: Path,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            action_path: input.parse()?,
            _comma: input.parse()?,
            handler_fn: input.parse()?,
        })
    }
}

pub fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: Input = syn::parse2(input)?;

    let segments: Vec<_> = input.action_path.segments.iter().collect();
    if segments.len() < 2 {
        return Err(syn::Error::new_spanned(
            &input.action_path,
            "impl_handler! requires a qualified path: module::ActionName",
        ));
    }

    let action_name = &segments.last().unwrap().ident;
    let trait_name = format_ident!("__{}Handler", action_name);

    let leading = &input.action_path.leading_colon;
    let module_segs: Vec<&syn::PathSegment> = segments[..segments.len() - 1].to_vec();
    let handler_fn = &input.handler_fn;

    Ok(quote! {
        impl #leading #(#module_segs)::* :: #trait_name
            for #leading #(#module_segs)::* :: __Handlers
        {
            fn route(kv: std::sync::Arc<dyn openerp_kv::KVStore>) -> axum::Router {
                #handler_fn(kv)
            }
        }
    })
}
