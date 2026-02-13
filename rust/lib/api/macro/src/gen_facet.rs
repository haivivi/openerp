//! Code generation for `#[facet]`.
//!
//! Generates:
//! 1. The facet struct with Serialize/Deserialize
//! 2. A router function that creates Axum routes with CRUD handlers
//! 3. Permission checks via Authenticator trait

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

    let doc_attrs: Vec<_> = item.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();
    let pass_through_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| !a.path().is_ident("facet") && !a.path().is_ident("doc") && !a.path().is_ident("readonly"))
        .collect();

    // Strip #[readonly] from fields.
    let mut clean_fields = item.fields.clone();
    if let syn::Fields::Named(ref mut named) = clean_fields {
        for field in named.named.iter_mut() {
            field.attrs.retain(|a| !a.path().is_ident("readonly"));
        }
    }

    let model_name = &ir.model;
    let store_name = format_ident!("{}Store", model_name);
    let resource_snake = to_snake_case(model_name);
    let list_path = format!("/{}", pluralize(&resource_snake));
    let item_path = format!("/{}/{{id}}", pluralize(&resource_snake));

    // Permission strings.
    let perm_create = format!("{}:create", resource_snake);
    let perm_read = format!("{}:read", resource_snake);
    let perm_list = format!("{}:list", resource_snake);
    let perm_update = format!("{}:update", resource_snake);
    let perm_delete = format!("{}:delete", resource_snake);

    // Router function name.
    let router_fn_name = format_ident!(
        "{}_{}_{}_router",
        to_snake_case(&ir.facet),
        resource_snake,
        ""
    );
    // Cleaner name:
    let router_fn_name = format_ident!("{}_{}_router", ir.facet, resource_snake);

    let crud_router = if ir.crud {
        quote! {
            /// Build Axum router for this facet's CRUD endpoints.
            #vis fn #router_fn_name(
                store: std::sync::Arc<#store_name>,
                auth: std::sync::Arc<dyn openerp_core::Authenticator>,
                module_id: &str,
            ) -> axum::Router {
                use axum::{Router, routing::{get, post, put, delete}, extract::{Path, State, Query}, Json};
                use openerp_core::{ServiceError, ListResult};

                type AppState = (
                    std::sync::Arc<#store_name>,
                    std::sync::Arc<dyn openerp_core::Authenticator>,
                    String, // module_id prefix for permissions
                );

                let state: AppState = (store, auth, module_id.to_string());

                async fn list_handler(
                    State((store, auth, module_id)): State<AppState>,
                    headers: axum::http::HeaderMap,
                ) -> Result<Json<ListResult<#facet_struct_name>>, ServiceError> {
                    let perm = format!("{}:{}", module_id, #perm_list);
                    auth.check(&headers, &perm)?;
                    let records = store.list()?;
                    let items: Vec<#facet_struct_name> = records.iter().map(|r| {
                        let json = serde_json::to_value(r).unwrap();
                        serde_json::from_value(json).unwrap()
                    }).collect();
                    let total = items.len();
                    Ok(Json(ListResult { items, total }))
                }

                async fn get_handler(
                    State((store, auth, module_id)): State<AppState>,
                    Path(id): Path<String>,
                    headers: axum::http::HeaderMap,
                ) -> Result<Json<#facet_struct_name>, ServiceError> {
                    let perm = format!("{}:{}", module_id, #perm_read);
                    auth.check(&headers, &perm)?;
                    let record = store.get_or_err(&id)?;
                    let json = serde_json::to_value(&record).unwrap();
                    let facet: #facet_struct_name = serde_json::from_value(json)
                        .map_err(|e| ServiceError::Internal(e.to_string()))?;
                    Ok(Json(facet))
                }

                async fn create_handler(
                    State((store, auth, module_id)): State<AppState>,
                    headers: axum::http::HeaderMap,
                    Json(value): Json<serde_json::Value>,
                ) -> Result<Json<#facet_struct_name>, ServiceError> {
                    let perm = format!("{}:{}", module_id, #perm_create);
                    auth.check(&headers, &perm)?;
                    let record = serde_json::from_value(value)
                        .map_err(|e| ServiceError::Validation(e.to_string()))?;
                    let created = store.create(record)?;
                    let json = serde_json::to_value(&created).unwrap();
                    let facet: #facet_struct_name = serde_json::from_value(json)
                        .map_err(|e| ServiceError::Internal(e.to_string()))?;
                    Ok(Json(facet))
                }

                async fn update_handler(
                    State((store, auth, module_id)): State<AppState>,
                    Path(id): Path<String>,
                    headers: axum::http::HeaderMap,
                    Json(value): Json<serde_json::Value>,
                ) -> Result<Json<#facet_struct_name>, ServiceError> {
                    let perm = format!("{}:{}", module_id, #perm_update);
                    auth.check(&headers, &perm)?;
                    let record = serde_json::from_value(value)
                        .map_err(|e| ServiceError::Validation(e.to_string()))?;
                    let updated = store.update(&id, record)?;
                    let json = serde_json::to_value(&updated).unwrap();
                    let facet: #facet_struct_name = serde_json::from_value(json)
                        .map_err(|e| ServiceError::Internal(e.to_string()))?;
                    Ok(Json(facet))
                }

                async fn delete_handler(
                    State((store, auth, module_id)): State<AppState>,
                    Path(id): Path<String>,
                    headers: axum::http::HeaderMap,
                ) -> Result<(), ServiceError> {
                    let perm = format!("{}:{}", module_id, #perm_delete);
                    auth.check(&headers, &perm)?;
                    store.delete(&id)?;
                    Ok(())
                }

                Router::new()
                    .route(#list_path, get(list_handler).post(create_handler))
                    .route(#item_path, get(get_handler).put(update_handler).delete(delete_handler))
                    .with_state(state)
            }
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #(#doc_attrs)*
        #(#pass_through_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #vis struct #facet_struct_name #clean_fields

        impl #facet_struct_name {
            pub const __DSL_FACET_IR: &'static str = #ir_json;
        }

        #crud_router
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
