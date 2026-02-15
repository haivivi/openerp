//! `#[request("path")]` macro expansion.
//!
//! Same as `#[state]` but derives `Debug, Clone` (no PartialEq).

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn expand(attr: TokenStream, item: ItemStruct) -> syn::Result<TokenStream> {
    let path = parse_path(attr)?;

    let struct_name = &item.ident;
    let vis = &item.vis;

    let doc_attrs: Vec<_> = item.attrs.iter()
        .filter(|a| a.path().is_ident("doc"))
        .collect();

    let user_derives = collect_derives(&item);
    let mut extra_derives = Vec::new();
    if !user_derives.contains(&"Debug".to_string()) {
        extra_derives.push(quote!(Debug));
    }
    if !user_derives.contains(&"Clone".to_string()) {
        extra_derives.push(quote!(Clone));
    }

    let user_derive_attrs: Vec<_> = item.attrs.iter()
        .filter(|a| a.path().is_ident("derive"))
        .collect();
    let other_attrs: Vec<_> = item.attrs.iter()
        .filter(|a| !a.path().is_ident("derive") && !a.path().is_ident("doc"))
        .collect();

    let derive_attr = if extra_derives.is_empty() && !user_derive_attrs.is_empty() {
        quote! { #(#user_derive_attrs)* }
    } else if user_derive_attrs.is_empty() {
        quote! { #[derive(#(#extra_derives),*)] }
    } else {
        let existing_tokens: Vec<_> = user_derive_attrs.iter().map(|a| quote!(#a)).collect();
        quote! {
            #(#existing_tokens)*
            #[derive(#(#extra_derives),*)]
        }
    };

    // Handle named fields, tuple fields, and unit structs.
    let fields = &item.fields;
    let struct_body = match fields {
        syn::Fields::Named(_) => quote! { #fields },
        syn::Fields::Unnamed(_) => {
            let semi = item.semi_token.map(|_| quote!(;)).unwrap_or_default();
            quote! { #fields #semi }
        }
        syn::Fields::Unit => quote! { ; },
    };

    Ok(quote! {
        #(#doc_attrs)*
        #derive_attr
        #(#other_attrs)*
        #vis struct #struct_name #struct_body

        impl #struct_name {
            /// The request path for Flux routing.
            pub const PATH: &'static str = #path;
        }
    })
}

fn parse_path(attr: TokenStream) -> syn::Result<String> {
    let lit: syn::LitStr = syn::parse2(attr)?;
    let path = lit.value();
    if path.is_empty() {
        return Err(syn::Error::new(lit.span(), "request path cannot be empty"));
    }
    Ok(path)
}

fn collect_derives(item: &ItemStruct) -> Vec<String> {
    let mut derives = Vec::new();
    for attr in &item.attrs {
        if attr.path().is_ident("derive") {
            if let Ok(meta) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            ) {
                for path in meta {
                    if let Some(ident) = path.get_ident() {
                        derives.push(ident.to_string());
                    }
                }
            }
        }
    }
    derives
}
