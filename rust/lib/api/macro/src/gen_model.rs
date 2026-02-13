//! Code generation for `#[model]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use openerp_dsl_parser::model_parser;

pub fn expand_model(attr: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    // The `#[model(module = "auth")]` attribute is passed as `attr` (not on item.attrs).
    // Inject it as a synthetic attribute so the parser can find it.
    let model_attr: syn::Attribute = syn::parse_quote!(#[model(#attr)]);
    item.attrs.insert(0, model_attr);

    // Parse into IR (validates key fields exist, module is set, etc.).
    let ir = model_parser::parse_model(&item)?;

    // Serialize IR to JSON for embedding as a const.
    let ir_json = serde_json::to_string(&ir).map_err(|e| {
        syn::Error::new_spanned(&item.ident, format!("failed to serialize model IR: {}", e))
    })?;

    let struct_name = &item.ident;
    let vis = &item.vis;

    // Strip #[ui(...)] from fields — only used by the parser.
    let mut clean_fields = item.fields.clone();
    if let syn::Fields::Named(ref mut named) = clean_fields {
        for field in named.named.iter_mut() {
            field.attrs.retain(|a| !a.path().is_ident("ui"));
        }
    }

    // Collect non-DSL attributes to re-emit (skip model, key).
    let pass_through_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| {
            !a.path().is_ident("model") && !a.path().is_ident("key") && !a.path().is_ident("doc")
        })
        .collect();

    // Collect doc attributes.
    let doc_attrs: Vec<_> = item.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();

    // Re-emit struct with derives + embedded IR.
    Ok(quote! {
        #(#doc_attrs)*
        #(#pass_through_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
        #[serde(rename_all = "camelCase")]
        #vis struct #struct_name #clean_fields

        impl #struct_name {
            /// Embedded IR metadata (JSON). Used by codegen and other macros.
            pub const __DSL_IR: &'static str = #ir_json;
        }
    })
}
