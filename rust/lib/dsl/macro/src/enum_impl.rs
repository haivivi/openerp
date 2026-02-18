//! `#[dsl_enum]` macro expansion.
//!
//! Generates: Serialize/Deserialize (SCREAMING_SNAKE_CASE), Display, FromStr,
//! Default (first variant), DslEnum impl, and a static `variants()` list.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemEnum, Lit};

pub fn expand(attr: TokenStream, item: ItemEnum) -> syn::Result<TokenStream> {
    let module = parse_module_attr(attr)?;

    let enum_name = &item.ident;
    let enum_name_str = enum_name.to_string();
    let vis = &item.vis;

    let doc_attrs: Vec<_> = item.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();
    let pass_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc") && !a.path().is_ident("dsl_enum"))
        .collect();

    if item.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            enum_name,
            "dsl_enum requires at least one variant",
        ));
    }

    for v in &item.variants {
        if !v.fields.is_empty() {
            return Err(syn::Error::new_spanned(
                v,
                "dsl_enum variants must be unit (no fields)",
            ));
        }
    }

    let variant_idents: Vec<_> = item.variants.iter().map(|v| &v.ident).collect();
    let variant_screaming: Vec<String> = variant_idents
        .iter()
        .map(|id| pascal_to_screaming_snake(&id.to_string()))
        .collect();

    let first_variant = &variant_idents[0];

    // Display: variant → SCREAMING_SNAKE_CASE string
    let display_arms: Vec<_> = variant_idents
        .iter()
        .zip(variant_screaming.iter())
        .map(|(ident, s)| quote! { Self::#ident => f.write_str(#s) })
        .collect();

    // FromStr: SCREAMING_SNAKE_CASE → variant (case-insensitive)
    let from_str_arms: Vec<_> = variant_idents
        .iter()
        .zip(variant_screaming.iter())
        .map(|(ident, s)| {
            let lower = s.to_ascii_lowercase();
            quote! { #lower => Ok(Self::#ident) }
        })
        .collect();

    let variants_array = &variant_screaming;
    let variant_count = variants_array.len();

    Ok(quote! {
        #(#doc_attrs)*
        #(#pass_attrs)*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        #vis enum #enum_name {
            #(#variant_idents),*
        }

        impl ::std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl ::std::str::FromStr for #enum_name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_ascii_lowercase().as_str() {
                    #(#from_str_arms,)*
                    _ => Err(format!("unknown {} variant: {}", #enum_name_str, s)),
                }
            }
        }

        impl Default for #enum_name {
            fn default() -> Self {
                Self::#first_variant
            }
        }

        impl openerp_types::DslEnum for #enum_name {
            fn module() -> &'static str { #module }
            fn enum_name() -> &'static str { #enum_name_str }
            fn variants() -> &'static [&'static str] {
                &[#(#variants_array),*]
            }
        }

        impl #enum_name {
            pub const __DSL_MODULE: &'static str = #module;
            pub const __DSL_ENUM_NAME: &'static str = #enum_name_str;

            pub fn variants() -> &'static [&'static str] {
                const V: [&str; #variant_count] = [#(#variants_array),*];
                &V
            }

            pub fn __dsl_ir() -> serde_json::Value {
                serde_json::json!({
                    "type": "enum",
                    "name": #enum_name_str,
                    "module": #module,
                    "variants": [#(#variants_array),*]
                })
            }
        }
    })
}

/// PascalCase → SCREAMING_SNAKE_CASE.
/// "InProgress" → "IN_PROGRESS", "Draft" → "DRAFT"
fn pascal_to_screaming_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_uppercase());
    }
    result
}

fn parse_module_attr(attr: TokenStream) -> syn::Result<String> {
    struct AttrArgs(Vec<syn::Meta>);
    impl syn::parse::Parse for AttrArgs {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let parsed =
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(
                    input,
                )?;
            Ok(Self(parsed.into_iter().collect()))
        }
    }

    let args: AttrArgs = syn::parse2(attr)?;
    for meta in &args.0 {
        if let syn::Meta::NameValue(nv) = meta {
            if nv.path.is_ident("module") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    return Ok(s.value());
                }
            }
        }
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "dsl_enum requires: #[dsl_enum(module = \"...\")]",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_to_screaming() {
        assert_eq!(pascal_to_screaming_snake("Draft"), "DRAFT");
        assert_eq!(pascal_to_screaming_snake("InProgress"), "IN_PROGRESS");
        assert_eq!(pascal_to_screaming_snake("Completed"), "COMPLETED");
        assert_eq!(pascal_to_screaming_snake("Active"), "ACTIVE");
        assert_eq!(pascal_to_screaming_snake("WiFiReady"), "WI_FI_READY");
    }
}
