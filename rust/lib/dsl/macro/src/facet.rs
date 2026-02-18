//! `#[facet]` macro expansion.
//!
//! Processes a module annotated with `#[facet(name = "mfg", module = "pms")]`.
//!
//! Inside the module:
//!   - `#[resource(path = "/models", pk = "code")]` on structs
//!     → serde-derived projection type
//!   - `#[action(method = "POST", path = "/batches/{id}/@provision")]` on type aliases
//!     → action metadata (fn pointer describes params + return type)
//!
//! Generates:
//!   - Modified module with serde-derived structs (resource)
//!   - Facet metadata consts (__FACET_NAME, __FACET_MODULE, __facet_ir)
//!   - Typed HTTP client struct ({Name}Client) using openerp_client

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Item, ItemMod, ItemStruct, ItemType, Lit};

// ── Parsed data ─────────────────────────────────────────────────────

struct FacetAttrs {
    name: String,
    module: String,
}

struct ResourceInfo {
    struct_name: syn::Ident,
    path: String,
    pk: String,
    /// Explicit singular form (e.g. "batch" for "/batches").
    /// If None, defaults to stripping trailing 's'.
    singular: Option<String>,
}

struct ActionParam {
    name: syn::Ident,
    ty: syn::Type,
    is_path_param: bool,
}

struct ActionInfo {
    type_name: syn::Ident,
    method: String,
    path: String,
    params: Vec<ActionParam>,
    return_type: Option<syn::Type>,
}

// ── Main expansion ──────────────────────────────────────────────────

pub fn expand(attr: TokenStream, item: ItemMod) -> syn::Result<TokenStream> {
    let facet = parse_facet_attrs(attr)?;
    let mod_ident = &item.ident;
    let vis = &item.vis;

    // Keep non-facet attrs (doc comments, etc.).
    let mod_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("facet"))
        .collect();

    let (_, items) = item.content.ok_or_else(|| {
        syn::Error::new_spanned(
            mod_ident,
            "#[facet] requires inline module: `mod name { ... }`, not `mod name;`",
        )
    })?;

    let mut resources = Vec::new();
    let mut actions = Vec::new();
    let mut output_items: Vec<TokenStream> = Vec::new();

    for item in &items {
        match item {
            Item::Struct(s) if has_attr(&s.attrs, "resource") => {
                let info = parse_resource(s)?;
                resources.push(info);
                output_items.push(emit_resource_struct(s));
                output_items.push(crate::flatbuf::emit_flatbuffer_impls(s));
            }
            Item::Type(t) if has_attr(&t.attrs, "action") => {
                let info = parse_action(t)?;
                actions.push(info);
                // Action type alias is consumed as metadata — not emitted.
            }
            other => {
                output_items.push(quote! { #other });
            }
        }
    }

    let facet_name = &facet.name;
    let facet_module = &facet.module;

    // ── Metadata IR ──

    let res_ir = resources.iter().map(|r| {
        let n = r.struct_name.to_string();
        let p = &r.path;
        let k = &r.pk;
        quote! { serde_json::json!({"name": #n, "path": #p, "pk": #k}) }
    });

    let act_ir = actions.iter().map(|a| {
        let n = fn_name_from_type(&a.type_name);
        let m = &a.method;
        let p = &a.path;
        // DELETE never sends a body, regardless of fn signature.
        let has_body = a.method != "DELETE" && a.params.iter().any(|p| !p.is_path_param);
        quote! { serde_json::json!({"name": #n, "method": #m, "path": #p, "hasBody": #has_body}) }
    });

    // ── Client struct ──

    let client_ident = format_ident!("{}Client", to_pascal_case(facet_name));
    let resource_methods: Vec<TokenStream> = resources
        .iter()
        .map(|r| emit_resource_client_methods(facet_name, facet_module, r))
        .collect();
    let action_methods: Vec<TokenStream> = actions
        .iter()
        .map(|a| emit_action_client_method(facet_name, facet_module, a))
        .collect();

    let client_doc = format!(
        "Auto-generated typed HTTP client for the `{}` facet.",
        facet_name
    );

    Ok(quote! {
        #(#mod_attrs)*
        #vis mod #mod_ident {
            #(#output_items)*

            // ── Metadata ──

            /// Facet name.
            pub const __FACET_NAME: &str = #facet_name;
            /// Facet module.
            pub const __FACET_MODULE: &str = #facet_module;

            /// Facet IR as JSON.
            pub fn __facet_ir() -> serde_json::Value {
                serde_json::json!({
                    "name": #facet_name,
                    "module": #facet_module,
                    "resources": [#(#res_ir),*],
                    "actions": [#(#act_ir),*],
                })
            }

            // ── Client ──

            #[doc = #client_doc]
            pub struct #client_ident {
                base: openerp_client::FacetClientBase,
            }

            impl #client_ident {
                /// Create a new facet client.
                ///
                /// `base_url` — server root (e.g. "http://localhost:8080").
                /// `token_source` — provides authentication tokens.
                pub fn new(
                    base_url: &str,
                    token_source: std::sync::Arc<dyn openerp_client::TokenSource>,
                ) -> Self {
                    Self {
                        base: openerp_client::FacetClientBase::new(base_url, token_source),
                    }
                }

                /// Set the preferred wire format for resource operations.
                ///
                /// `Format::FlatBuffers` enables zero-copy binary responses
                /// for `list_*` and `get_*` methods. Actions always use JSON.
                pub fn format(mut self, format: openerp_types::Format) -> Self {
                    self.base = self.base.with_format(format);
                    self
                }

                #(#resource_methods)*
                #(#action_methods)*
            }
        }
    })
}

// ── Attribute parsing ───────────────────────────────────────────────

fn parse_facet_attrs(attr: TokenStream) -> syn::Result<FacetAttrs> {
    struct Args(Vec<syn::Meta>);
    impl syn::parse::Parse for Args {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let p = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(
                input,
            )?;
            Ok(Self(p.into_iter().collect()))
        }
    }

    let args: Args = syn::parse2(attr)?;
    let mut name = None;
    let mut module = None;

    for meta in &args.0 {
        if let syn::Meta::NameValue(nv) = meta {
            if nv.path.is_ident("name") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    name = Some(s.value());
                }
            } else if nv.path.is_ident("module") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    module = Some(s.value());
                }
            }
        }
    }

    Ok(FacetAttrs {
        name: name.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[facet] requires: name = \"...\"",
            )
        })?,
        module: module.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[facet] requires: module = \"...\"",
            )
        })?,
    })
}

fn parse_resource(s: &ItemStruct) -> syn::Result<ResourceInfo> {
    let mut path = None;
    let mut pk = None;
    let mut singular = None;

    for attr in &s.attrs {
        if attr.path().is_ident("resource") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("path") {
                    let v = meta.value()?;
                    let lit: Lit = v.parse()?;
                    if let Lit::Str(s) = lit {
                        path = Some(s.value());
                    }
                } else if meta.path.is_ident("pk") {
                    let v = meta.value()?;
                    let lit: Lit = v.parse()?;
                    if let Lit::Str(s) = lit {
                        pk = Some(s.value());
                    }
                } else if meta.path.is_ident("singular") {
                    let v = meta.value()?;
                    let lit: Lit = v.parse()?;
                    if let Lit::Str(s) = lit {
                        singular = Some(s.value());
                    }
                }
                Ok(())
            })?;
        }
    }

    Ok(ResourceInfo {
        struct_name: s.ident.clone(),
        path: path.ok_or_else(|| {
            syn::Error::new_spanned(&s.ident, "#[resource] requires: path = \"/...\"")
        })?,
        pk: pk.ok_or_else(|| {
            syn::Error::new_spanned(&s.ident, "#[resource] requires: pk = \"...\"")
        })?,
        singular,
    })
}

fn parse_action(t: &ItemType) -> syn::Result<ActionInfo> {
    let mut method = String::new();
    let mut path = String::new();

    for attr in &t.attrs {
        if attr.path().is_ident("action") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("method") {
                    let v = meta.value()?;
                    let lit: Lit = v.parse()?;
                    if let Lit::Str(s) = lit {
                        method = s.value();
                    }
                } else if meta.path.is_ident("path") {
                    let v = meta.value()?;
                    let lit: Lit = v.parse()?;
                    if let Lit::Str(s) = lit {
                        path = s.value();
                    }
                }
                Ok(())
            })?;
        }
    }

    if method.is_empty() {
        return Err(syn::Error::new_spanned(
            &t.ident,
            "#[action] requires: method = \"POST\"",
        ));
    }
    if path.is_empty() {
        return Err(syn::Error::new_spanned(
            &t.ident,
            "#[action] requires: path = \"/...\"",
        ));
    }

    // Parse fn pointer type: fn(id: String, req: Request) -> Response
    let bare_fn = match t.ty.as_ref() {
        syn::Type::BareFn(bf) => bf,
        _ => {
            return Err(syn::Error::new_spanned(
                &t.ty,
                "#[action] type must be a fn pointer: `type Name = fn(...) -> Resp;`",
            ))
        }
    };

    // Extract path parameter names from URL template: /batches/{id}/@provision → ["id"]
    let path_param_names = extract_path_params(&path);

    let mut params = Vec::new();
    for arg in &bare_fn.inputs {
        let name = arg
            .name
            .as_ref()
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    &arg.ty,
                    "action fn params must be named: `fn(id: String, ...)`",
                )
            })?
            .0
            .clone();
        let is_path = path_param_names.contains(&name.to_string());
        params.push(ActionParam {
            name,
            ty: arg.ty.clone(),
            is_path_param: is_path,
        });
    }

    // Validate: every URL template param must have a matching fn param.
    let fn_param_names: Vec<String> = params.iter().map(|p| p.name.to_string()).collect();
    for url_param in &path_param_names {
        if !fn_param_names.contains(url_param) {
            return Err(syn::Error::new_spanned(
                &t.ident,
                format!(
                    "URL template has {{{}}} but fn signature has no parameter named `{}`",
                    url_param, url_param
                ),
            ));
        }
    }

    let return_type = match &bare_fn.output {
        syn::ReturnType::Type(_, ty) => Some(ty.as_ref().clone()),
        syn::ReturnType::Default => None,
    };

    Ok(ActionInfo {
        type_name: t.ident.clone(),
        method,
        path,
        params,
        return_type,
    })
}

// ── Code emission ───────────────────────────────────────────────────

/// Transform a `#[resource]` struct: strip the attribute, add serde derives.
fn emit_resource_struct(s: &ItemStruct) -> TokenStream {
    let vis = &s.vis;
    let ident = &s.ident;
    let fields = &s.fields;

    // Keep doc comments and pass-through attrs (not resource, not derive, not serde).
    let doc_attrs: Vec<_> = s.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();
    let pass_attrs: Vec<_> = s
        .attrs
        .iter()
        .filter(|a| {
            !a.path().is_ident("resource")
                && !a.path().is_ident("doc")
                && !a.path().is_ident("derive")
                && !a.path().is_ident("serde")
        })
        .collect();

    quote! {
        #(#doc_attrs)*
        #(#pass_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #vis struct #ident #fields
    }
}

/// Generate list + get client methods for a resource.
fn emit_resource_client_methods(
    facet_name: &str,
    facet_module: &str,
    res: &ResourceInfo,
) -> TokenStream {
    let struct_name = &res.struct_name;
    let path_segment = res.path.trim_start_matches('/');
    let singular = match &res.singular {
        Some(s) => s.clone(),
        None => path_segment.strip_suffix('s').unwrap_or(path_segment).to_string(),
    };
    let list_fn = format_ident!("list_{}", path_segment);
    let get_fn = format_ident!("get_{}", singular);
    let pk_ident = format_ident!("{}", res.pk);

    let list_path = format!("/{}/{}{}", facet_name, facet_module, res.path);
    let get_path_fmt = format!("/{}/{}{}/{{}}", facet_name, facet_module, res.path);

    let list_doc = format!("List all {} — GET {}", path_segment, res.path);
    let get_doc = format!("Get {} by {} — GET {}/{{{}}}", singular, res.pk, res.path, res.pk);

    quote! {
        #[doc = #list_doc]
        pub async fn #list_fn(&self, params: Option<&openerp_client::ListParams>) -> Result<openerp_client::ListResult<#struct_name>, openerp_client::ApiError> {
            let mut url = #list_path.to_string();
            if let Some(p) = params {
                let mut parts = Vec::new();
                if let Some(limit) = p.limit { parts.push(format!("limit={}", limit)); }
                if let Some(offset) = p.offset { parts.push(format!("offset={}", offset)); }
                if !parts.is_empty() {
                    url.push('?');
                    url.push_str(&parts.join("&"));
                }
            }
            self.base.list(&url).await
        }

        #[doc = #get_doc]
        pub async fn #get_fn(&self, #pk_ident: &str) -> Result<#struct_name, openerp_client::ApiError> {
            self.base.get(&format!(#get_path_fmt, #pk_ident)).await
        }
    }
}

/// Generate a client method for an action.
fn emit_action_client_method(
    facet_name: &str,
    facet_module: &str,
    action: &ActionInfo,
) -> TokenStream {
    let fn_name = format_ident!("{}", fn_name_from_type(&action.type_name));

    // Build format string: /mfg/pms/batches/{}/@provision
    let path_fmt = build_path_format(facet_name, facet_module, &action.path);

    let body_param: Option<&ActionParam> = action.params.iter().find(|p| !p.is_path_param);
    let is_delete = action.method == "DELETE";

    // DELETE actions must not have a body parameter.
    // Filter body params out of fn_args for DELETE to avoid a misleading signature.
    let fn_args: Vec<TokenStream> = action
        .params
        .iter()
        .filter(|p| !is_delete || p.is_path_param)
        .map(|p| {
            let name = &p.name;
            if p.is_path_param {
                quote! { #name: &str }
            } else {
                let ty = &p.ty;
                quote! { #name: &#ty }
            }
        })
        .collect();

    // format! arguments ordered by URL template position, not fn signature.
    // extract_path_params returns names in URL order: /orgs/{org_id}/users/{user_id} → ["org_id", "user_id"]
    let url_param_order = extract_path_params(&action.path);
    let format_args: Vec<syn::Ident> = url_param_order
        .iter()
        .filter_map(|name| {
            action.params.iter().find(|p| p.name == name).map(|p| p.name.clone())
        })
        .collect();

    // DELETE always returns () — no response body.
    let ret_ty = if is_delete {
        quote! { () }
    } else {
        match &action.return_type {
            Some(ty) => quote! { #ty },
            None => quote! { () },
        }
    };

    let effective_body = if is_delete { None } else { body_param };
    let call = match (action.method.as_str(), effective_body) {
        ("POST", Some(body)) => {
            let body_name = &body.name;
            quote! { self.base.post(&path, #body_name).await }
        }
        ("POST", None) => quote! { self.base.post_empty(&path).await },
        ("PUT", Some(body)) => {
            let body_name = &body.name;
            quote! { self.base.put(&path, #body_name).await }
        }
        ("PUT", None) => quote! { self.base.put_empty(&path).await },
        ("DELETE", _) => quote! { self.base.delete(&path).await },
        _ => {
            if let Some(body) = body_param {
                let body_name = &body.name;
                quote! { self.base.post(&path, #body_name).await }
            } else {
                quote! { self.base.post_empty(&path).await }
            }
        }
    };

    let doc = format!(
        "{} {}",
        action.method,
        action.path
    );

    quote! {
        #[doc = #doc]
        pub async fn #fn_name(&self, #(#fn_args),*) -> Result<#ret_ty, openerp_client::ApiError> {
            let path = format!(#path_fmt, #(#format_args),*);
            #call
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}

/// Extract path parameter names from a URL template.
/// "/batches/{id}/@provision" → ["id"]
fn extract_path_params(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        if let Some(end) = rest[start..].find('}') {
            params.push(rest[start + 1..start + end].to_string());
            rest = &rest[start + end + 1..];
        } else {
            break;
        }
    }
    params
}

/// Build a format string for the client URL.
/// Prepends /{facet}/{module} and replaces {param} with {}.
fn build_path_format(facet_name: &str, facet_module: &str, action_path: &str) -> String {
    let prefix = format!("/{}/{}", facet_name, facet_module);
    let mut result = prefix;
    let mut rest = action_path;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find('}') {
            result.push_str("{}");
            rest = &rest[start + end + 1..];
        } else {
            result.push('{');
            rest = &rest[start + 1..];
        }
    }
    result.push_str(rest);
    result
}

/// Derive fn name from type name: Provision → provision, ActivateDevice → activate_device
fn fn_name_from_type(ident: &syn::Ident) -> String {
    to_snake_case(&ident.to_string())
}

fn to_snake_case(s: &str) -> String {
    crate::util::to_snake_case(s)
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_path_params() {
        assert_eq!(
            extract_path_params("/batches/{id}/@provision"),
            vec!["id"]
        );
        assert_eq!(
            extract_path_params("/devices/{sn}/@activate"),
            vec!["sn"]
        );
        assert_eq!(
            extract_path_params("/a/{x}/b/{y}/c"),
            vec!["x", "y"]
        );
        assert!(extract_path_params("/models").is_empty());
    }

    #[test]
    fn test_build_path_format() {
        assert_eq!(
            build_path_format("mfg", "pms", "/batches/{id}/@provision"),
            "/mfg/pms/batches/{}/@provision"
        );
        assert_eq!(
            build_path_format("mfg", "pms", "/devices/{sn}/@activate"),
            "/mfg/pms/devices/{}/@activate"
        );
        assert_eq!(
            build_path_format("mfg", "pms", "/models"),
            "/mfg/pms/models"
        );
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("mfg"), "Mfg");
        assert_eq!(to_pascal_case("my_facet"), "MyFacet");
        assert_eq!(to_pascal_case("app"), "App");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Provision"), "provision");
        assert_eq!(to_snake_case("ActivateDevice"), "activate_device");
        assert_eq!(to_snake_case("Upload"), "upload");
    }
}
