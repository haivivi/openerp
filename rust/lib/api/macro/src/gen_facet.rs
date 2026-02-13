//! Code generation for `#[facet]`.
//!
//! Generates Axum router with CRUD routes for a facet.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemStruct;

use openerp_dsl_parser::parse_facet;

pub fn expand_facet(attr: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let facet_attr: syn::Attribute = syn::parse_quote!(#[facet(#attr)]);
    item.attrs.insert(0, facet_attr);

    let ir = parse_facet(&item)?;

    let ir_json = serde_json::to_string(&ir).map_err(|e| {
        syn::Error::new_spanned(&item.ident, format!("failed to serialize facet IR: {}", e))
    })?;

    let facet_struct_name = &item.ident;
    let vis = &item.vis;
    let fields = &item.fields;

    let doc_attrs: Vec<_> = item.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();
    let pass_through_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("facet") && !a.path().is_ident("doc"))
        .collect();

    let model_name = &ir.model;
    let store_name = format_ident!("{}Store", model_name);
    let router_fn = format_ident!("{}_router", to_snake_case(&ir.facet));

    let resource_snake = to_snake_case(model_name);
    let list_path = format!("/{}", pluralize(&resource_snake));
    let get_path = format!("/{}/:id", pluralize(&resource_snake));

    // Permission strings for CRUD.
    let module_placeholder = "MODULE"; // Will be set at module wire-up time.
    let perm_create = format!("{}:{}:create", module_placeholder, resource_snake);
    let perm_read = format!("{}:{}:read", module_placeholder, resource_snake);
    let perm_list = format!("{}:{}:list", module_placeholder, resource_snake);
    let perm_update = format!("{}:{}:update", module_placeholder, resource_snake);
    let perm_delete = format!("{}:{}:delete", module_placeholder, resource_snake);

    // Generate field projection: convert DB record to facet struct.
    let _field_projections: Vec<_> = ir
        .fields
        .iter()
        .map(|f| {
            let name = format_ident!("{}", f.name);
            quote! { #name: record.#name.clone() }
        })
        .collect();

    let _crud_routes = if ir.crud {
        quote! {
            .route(#list_path, axum::routing::get(list_handler).post(create_handler))
            .route(#get_path, axum::routing::get(get_handler).put(update_handler).delete(delete_handler))
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #(#doc_attrs)*
        #(#pass_through_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #vis struct #facet_struct_name #fields

        impl #facet_struct_name {
            pub const __DSL_FACET_IR: &'static str = #ir_json;

            /// Project a DB record into this facet's field subset.
            pub fn from_record(record: &<#store_name as std::ops::Deref>::Target) -> Self {
                // This is a simplified projection. In practice, the macro
                // would generate field-by-field mapping.
                // For now, we rely on serde for compatible structs.
                unimplemented!("facet projection generated at compile time")
            }
        }

        // The router function will be generated in Step 6 with proper handler
        // wiring. For now, we just emit the facet struct and metadata.
    })
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

fn pluralize(s: &str) -> String {
    if s.ends_with('s') || s.ends_with("sh") || s.ends_with("ch") || s.ends_with('x') {
        format!("{}es", s)
    } else if s.ends_with('y') && !s.ends_with("ey") {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}
