//! `#[model]` macro expansion.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Fields, ItemStruct, Lit};

pub fn expand(attr: TokenStream, item: ItemStruct) -> syn::Result<TokenStream> {
    // Parse module name from attr: #[model(module = "auth")]
    let module = parse_module_attr(attr)?;

    let struct_name = &item.ident;
    let struct_name_str = struct_name.to_string();
    let vis = &item.vis;

    // Collect doc attrs.
    let doc_attrs: Vec<_> = item.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();

    // Collect non-DSL attrs to pass through.
    let pass_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("doc") && !a.path().is_ident("model"))
        .collect();

    // Parse fields.
    let named = match &item.fields {
        Fields::Named(n) => n,
        _ => return Err(syn::Error::new_spanned(&item.ident, "model must have named fields")),
    };

    // Strip #[ui(...)] and #[permission(...)] from field output.
    // Add #[serde(default)] to all fields for flexible deserialization.
    let mut clean_fields = named.clone();
    for field in clean_fields.named.iter_mut() {
        field
            .attrs
            .retain(|a| !a.path().is_ident("ui") && !a.path().is_ident("permission"));
        let has_serde_default = field.attrs.iter().any(|a| {
            if a.path().is_ident("serde") {
                a.meta.to_token_stream().to_string().contains("default")
            } else {
                false
            }
        });
        if !has_serde_default {
            field.attrs.push(syn::parse_quote!(#[serde(default)]));
        }
    }

    // Inject common fields if not already present:
    // display_name, description, metadata, created_at, updated_at, rev
    let existing_names: Vec<String> = clean_fields
        .named
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
        .collect();

    let common_fields: Vec<(&str, syn::Type)> = vec![
        ("display_name", syn::parse_quote!(Option<String>)),
        ("description", syn::parse_quote!(Option<String>)),
        ("metadata", syn::parse_quote!(Option<String>)),
        ("created_at", syn::parse_quote!(openerp_types::DateTime)),
        ("updated_at", syn::parse_quote!(openerp_types::DateTime)),
    ];

    for (name, ty) in &common_fields {
        if !existing_names.contains(&name.to_string()) {
            let ident = format_ident!("{}", name);
            let field: syn::Field = syn::parse_quote! {
                #[serde(default)]
                pub #ident: #ty
            };
            clean_fields.named.push(field);
        }
    }

    // Generate Field consts and IR data for each field.
    let mut field_consts = Vec::new();
    let mut field_ir_entries = Vec::new();

    for field in &named.named {
        let fname = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must have a name"))?;
        let fname_str = fname.to_string();

        // Get the outermost type name for widget inference.
        let ty_str = type_to_string(&field.ty);
        let inner_ty = extract_inner_type_name(&field.ty);

        // Check for explicit #[ui(widget = "...")] override.
        let explicit_widget = extract_ui_widget(&field.attrs)?;

        let widget_str = match explicit_widget {
            Some(w) => w,
            None => infer_widget(&inner_ty, &fname_str).to_string(),
        };

        // Field const: pub const field_name: Field = Field::new("name", "Type", "widget");
        let const_name = format_ident!("{}", fname_str);
        field_consts.push(quote! {
            pub const #const_name: openerp_types::Field =
                openerp_types::Field::new(#fname_str, #ty_str, #widget_str);
        });

        // IR entry for schema JSON.
        field_ir_entries.push(quote! {
            serde_json::json!({
                "name": #fname_str,
                "ty": #ty_str,
                "widget": #widget_str
            })
        });
    }

    // Generate Field consts + IR for injected common fields.
    for (name, ty) in &common_fields {
        if !existing_names.contains(&name.to_string()) {
            let ty_str = quote!(#ty).to_string().replace(' ', "");
            let inner_ty = extract_inner_type_name(ty);
            let widget_str = infer_widget(&inner_ty, name).to_string();
            let const_name = format_ident!("{}", name);
            field_consts.push(quote! {
                pub const #const_name: openerp_types::Field =
                    openerp_types::Field::new(#name, #ty_str, #widget_str);
            });
            field_ir_entries.push(quote! {
                serde_json::json!({
                    "name": #name,
                    "ty": #ty_str,
                    "widget": #widget_str
                })
            });
        }
    }

    let resource_snake = to_snake_case(&struct_name_str);
    let resource_path = pluralize(&resource_snake);

    Ok(quote! {
        #(#doc_attrs)*
        #(#pass_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
        #[serde(rename_all = "camelCase")]
        #vis struct #struct_name #clean_fields

        impl #struct_name {
            // ── Field consts (compile-time checked references) ──
            #(#field_consts)*

            // ── DSL metadata ──
            pub const __DSL_MODULE: &'static str = #module;
            pub const __DSL_NAME: &'static str = #struct_name_str;
            pub const __DSL_RESOURCE: &'static str = #resource_snake;
            pub const __DSL_PATH: &'static str = #resource_path;

            /// All fields as an array.
            pub fn __dsl_fields() -> Vec<serde_json::Value> {
                vec![ #(#field_ir_entries),* ]
            }

            /// Full IR as JSON value.
            pub fn __dsl_ir() -> serde_json::Value {
                serde_json::json!({
                    "name": #struct_name_str,
                    "module": #module,
                    "resource": #resource_snake,
                    "fields": Self::__dsl_fields()
                })
            }
        }

        impl openerp_types::DslModel for #struct_name {
            fn module() -> &'static str { #module }
            fn resource() -> &'static str { #resource_snake }
            fn resource_path() -> &'static str { #resource_path }
        }
    })
}

fn parse_module_attr(attr: TokenStream) -> syn::Result<String> {
    // Parse: module = "auth"
    // Use a helper struct since Punctuated<Meta> doesn't impl Parse directly.
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
        "model requires: #[model(module = \"...\")]",
    ))
}

/// Get the full type as a string (e.g. "Option<Email>").
fn type_to_string(ty: &syn::Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

/// Extract the innermost meaningful type name for widget inference.
/// Option<Email> -> "Email", Vec<String> -> "Vec<String>", String -> "String"
fn extract_inner_type_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let name = seg.ident.to_string();
            if name == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return extract_inner_type_name(inner);
                    }
                }
            }
            return name;
        }
    }
    "String".to_string()
}

/// Extract #[ui(widget = "...")] from field attributes.
fn extract_ui_widget(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("ui") {
            let mut widget = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("widget") {
                    let v = meta.value()?;
                    let lit: Lit = v.parse()?;
                    if let Lit::Str(s) = lit {
                        widget = Some(s.value());
                    }
                }
                Ok(())
            })?;
            if let Some(w) = widget {
                return Ok(Some(w));
            }
        }
    }
    Ok(None)
}

/// Known DSL builtin type names — anything not in this set that starts
/// with an uppercase letter is assumed to be a `#[dsl_enum]` and gets
/// the `"select"` widget.
///
/// **Must stay in sync with `openerp_types::BUILTIN_TYPES`.**
/// Duplicated here because proc-macro crates cannot depend on runtime crates.
const BUILTIN_TYPES: &[&str] = &[
    "Id", "Email", "Phone", "Url", "Avatar", "ImageUrl",
    "Password", "PasswordHash", "Secret",
    "Text", "Markdown", "Code",
    "DateTime", "Date", "Color", "SemVer",
    "String", "bool", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64",
    "f32", "f64", "Vec",
];

fn infer_widget(ty_name: &str, field_name: &str) -> &'static str {
    match ty_name {
        "Id" => "readonly",
        "Email" => "email",
        "Phone" => "tel",
        "Url" => "url",
        "Avatar" | "ImageUrl" => "image",
        "Password" => "password",
        "PasswordHash" | "Secret" => "hidden",
        "Text" => "textarea",
        "Markdown" => "markdown",
        "Code" => "code",
        "DateTime" => "datetime",
        "Date" => "date",
        "Color" => "color",
        "SemVer" => "text",
        "bool" => "switch",
        "Vec" => "tags",
        _ => {
            if field_name.ends_with("_at") {
                "datetime"
            } else if field_name == "description" || field_name == "notes" {
                "textarea"
            } else if is_enum_type(ty_name) {
                "select"
            } else {
                "text"
            }
        }
    }
}

/// Heuristic: a type name that starts uppercase and isn't a known builtin
/// is treated as a `#[dsl_enum]` → select widget.
fn is_enum_type(ty_name: &str) -> bool {
    ty_name.starts_with(|c: char| c.is_ascii_uppercase()) && !BUILTIN_TYPES.contains(&ty_name)
}

fn to_snake_case(s: &str) -> String {
    crate::util::to_snake_case(s)
}

/// Simple English pluralization for URL paths.
///
/// **Must stay in sync with `openerp_types::pluralize`.**
/// Duplicated here because proc-macro crates cannot depend on runtime crates.
/// The canonical version with tests lives in `openerp_types`.
fn pluralize(s: &str) -> String {
    if s.ends_with('y') {
        // Check if preceded by a consonant: policy -> policies
        let chars: Vec<char> = s.chars().collect();
        if chars.len() >= 2 {
            let before_y = chars[chars.len() - 2];
            if !"aeiou".contains(before_y) {
                return format!("{}ies", &s[..s.len() - 1]);
            }
        }
        format!("{}s", s)
    } else if s.ends_with('s') || s.ends_with('x') || s.ends_with('z')
        || s.ends_with("sh") || s.ends_with("ch")
    {
        format!("{}es", s)
    } else {
        format!("{}s", s)
    }
}
