//! Code generation for `#[persistent]`.
//!
//! Generates:
//! 1. The DB struct with Serialize/Deserialize
//! 2. A `{Model}Store` struct with CRUD methods using KVStore

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemStruct;

use openerp_dsl_parser::parse_persistent;
use openerp_ir::{AutoFill, FieldType, IndexKind};

pub fn expand_persistent(attr: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    // Inject the outer attribute back onto the struct for the parser.
    let persistent_attr: syn::Attribute = syn::parse_quote!(#[persistent(#attr)]);
    item.attrs.insert(0, persistent_attr);

    let ir = parse_persistent(&item)?;

    let ir_json = serde_json::to_string(&ir).map_err(|e| {
        syn::Error::new_spanned(&item.ident, format!("failed to serialize persistent IR: {}", e))
    })?;

    let db_struct_name = &item.ident;
    let vis = &item.vis;
    let fields = &item.fields;

    // Collect non-DSL attributes to re-emit.
    let pass_through_attrs: Vec<_> = item
        .attrs
        .iter()
        .filter(|a| {
            let p = a.path();
            !p.is_ident("persistent")
                && !p.is_ident("key")
                && !p.is_ident("unique")
                && !p.is_ident("index")
                && !p.is_ident("search")
                && !p.is_ident("filter")
                && !p.is_ident("doc")
        })
        .collect();

    let doc_attrs: Vec<_> = item.attrs.iter().filter(|a| a.path().is_ident("doc")).collect();

    // Strip #[auto(...)] from fields and add #[serde(default)] to auto-fill fields.
    let auto_field_names: Vec<String> = ir
        .fields
        .iter()
        .filter(|f| f.auto.is_some())
        .map(|f| f.name.clone())
        .collect();

    let mut clean_fields = item.fields.clone();
    if let syn::Fields::Named(ref mut named) = clean_fields {
        for field in named.named.iter_mut() {
            let field_name = field.ident.as_ref().map(|i| i.to_string()).unwrap_or_default();
            field.attrs.retain(|a| !a.path().is_ident("auto"));
            // Add #[serde(default)] to auto-fill fields so deserialization doesn't require them.
            if auto_field_names.contains(&field_name) {
                field.attrs.push(syn::parse_quote!(#[serde(default)]));
            }
        }
    }

    // Generate store struct name: User -> UserStore
    let model_name = &ir.model;
    let store_name = format_ident!("{}Store", model_name);

    // Key prefix for KV: "auth:user:" or "pms:model:" etc.
    let kv_prefix = format!("{}:", to_snake_case(model_name));

    // Generate key construction function.
    let key_fields = &ir.key.fields;
    let key_construction = if key_fields.len() == 1 {
        let kf = format_ident!("{}", &key_fields[0]);
        quote! {
            fn make_key(record: &#db_struct_name) -> String {
                format!("{}{}", Self::PREFIX, record.#kf)
            }
            fn make_key_from_id(id: &str) -> String {
                format!("{}{}", Self::PREFIX, id)
            }
        }
    } else {
        // Compound key: join with "/"
        let kf_idents: Vec<_> = key_fields.iter().map(|f| format_ident!("{}", f)).collect();
        let kf_formats: Vec<_> = key_fields.iter().map(|_| "{}").collect();
        let compound_fmt = kf_formats.join("/");
        let full_fmt = format!("{{}}{}", compound_fmt);
        quote! {
            fn make_key(record: &#db_struct_name) -> String {
                format!(#full_fmt, Self::PREFIX, #(record.#kf_idents),*)
            }
            fn make_key_from_id(id: &str) -> String {
                format!("{}{}", Self::PREFIX, id)
            }
        }
    };

    // Generate auto-fill logic for create.
    let auto_fill_create = generate_auto_fill_create(&ir);

    // Generate auto-fill logic for update.
    let auto_fill_update = generate_auto_fill_update(&ir);

    // Generate unique index checks.
    let unique_checks = generate_unique_checks(&ir);
    let unique_index_updates = generate_unique_index_updates(&ir);
    let unique_index_deletes = generate_unique_index_deletes(&ir);

    // Generate filter index updates.
    let filter_index_updates = generate_filter_index_updates(&ir);
    let filter_index_deletes = generate_filter_index_deletes(&ir);

    Ok(quote! {
        #(#doc_attrs)*
        #(#pass_through_attrs)*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #vis struct #db_struct_name #clean_fields

        impl #db_struct_name {
            pub const __DSL_PERSISTENT_IR: &'static str = #ir_json;
        }

        /// Auto-generated CRUD store for #model_name.
        #vis struct #store_name {
            kv: std::sync::Arc<dyn openerp_kv::KVStore>,
        }

        impl #store_name {
            const PREFIX: &'static str = #kv_prefix;

            pub fn new(kv: std::sync::Arc<dyn openerp_kv::KVStore>) -> Self {
                Self { kv }
            }

            #key_construction

            pub fn get(&self, id: &str) -> Result<Option<#db_struct_name>, openerp_core::ServiceError> {
                let key = Self::make_key_from_id(id);
                match self.kv.get(&key).map_err(Self::kv_err)? {
                    Some(bytes) => {
                        let record: #db_struct_name = serde_json::from_slice(&bytes)
                            .map_err(|e| openerp_core::ServiceError::Internal(e.to_string()))?;
                        Ok(Some(record))
                    }
                    None => Ok(None),
                }
            }

            pub fn get_or_err(&self, id: &str) -> Result<#db_struct_name, openerp_core::ServiceError> {
                self.get(id)?.ok_or_else(|| openerp_core::ServiceError::NotFound(
                    format!("{} '{}' not found", #model_name, id)
                ))
            }

            pub fn list(&self) -> Result<Vec<#db_struct_name>, openerp_core::ServiceError> {
                let prefix = Self::PREFIX;
                let entries = self.kv.scan(prefix)
                    .map_err(|e| openerp_core::ServiceError::Internal(e.to_string()))?;
                let mut records = Vec::with_capacity(entries.len());
                for (_key, bytes) in entries {
                    let record: #db_struct_name = serde_json::from_slice(&bytes)
                        .map_err(|e| openerp_core::ServiceError::Internal(e.to_string()))?;
                    records.push(record);
                }
                Ok(records)
            }

            fn kv_err(e: openerp_kv::KVError) -> openerp_core::ServiceError {
                match e {
                    openerp_kv::KVError::ReadOnly(msg) => openerp_core::ServiceError::ReadOnly(msg),
                    other => openerp_core::ServiceError::Storage(other.to_string()),
                }
            }

            pub fn create(&self, mut record: #db_struct_name) -> Result<#db_struct_name, openerp_core::ServiceError> {
                // Auto-fill fields.
                #auto_fill_create

                // Check unique constraints.
                #unique_checks

                let key = Self::make_key(&record);
                // Check if already exists.
                if self.kv.get(&key).map_err(Self::kv_err)?.is_some() {
                    return Err(openerp_core::ServiceError::Validation(
                        format!("{} already exists", #model_name)
                    ));
                }

                let bytes = serde_json::to_vec(&record)
                    .map_err(|e| openerp_core::ServiceError::Internal(e.to_string()))?;
                self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

                // Update indexes.
                #unique_index_updates
                #filter_index_updates

                Ok(record)
            }

            pub fn update(&self, id: &str, mut record: #db_struct_name) -> Result<#db_struct_name, openerp_core::ServiceError> {
                // Ensure exists.
                let _existing = self.get_or_err(id)?;

                // Auto-fill update timestamps.
                #auto_fill_update

                let key = Self::make_key_from_id(id);
                let bytes = serde_json::to_vec(&record)
                    .map_err(|e| openerp_core::ServiceError::Internal(e.to_string()))?;
                self.kv.set(&key, &bytes).map_err(Self::kv_err)?;

                Ok(record)
            }

            pub fn delete(&self, id: &str) -> Result<(), openerp_core::ServiceError> {
                let record = self.get_or_err(id)?;
                let key = Self::make_key_from_id(id);
                self.kv.delete(&key).map_err(Self::kv_err)?;

                // Clean up indexes.
                #unique_index_deletes
                #filter_index_deletes

                Ok(())
            }
        }
    })
}

fn generate_auto_fill_create(ir: &openerp_ir::PersistentIR) -> TokenStream {
    let mut stmts = Vec::new();
    for field in &ir.fields {
        let fname = format_ident!("{}", field.name);
        match &field.auto {
            Some(AutoFill::Uuid) => {
                if field.ty == FieldType::String {
                    stmts.push(quote! {
                        if record.#fname.is_empty() {
                            record.#fname = uuid::Uuid::new_v4().to_string();
                        }
                    });
                }
            }
            Some(AutoFill::CreateTimestamp) => {
                if field.ty == FieldType::String {
                    stmts.push(quote! {
                        if record.#fname.is_empty() {
                            record.#fname = chrono::Utc::now().to_rfc3339();
                        }
                    });
                } else if field.ty == FieldType::Option(Box::new(FieldType::String)) {
                    stmts.push(quote! {
                        if record.#fname.is_none() {
                            record.#fname = Some(chrono::Utc::now().to_rfc3339());
                        }
                    });
                }
            }
            Some(AutoFill::UpdateTimestamp) => {
                if field.ty == FieldType::String {
                    stmts.push(quote! {
                        record.#fname = chrono::Utc::now().to_rfc3339();
                    });
                } else if field.ty == FieldType::Option(Box::new(FieldType::String)) {
                    stmts.push(quote! {
                        record.#fname = Some(chrono::Utc::now().to_rfc3339());
                    });
                }
            }
            None => {}
        }
    }
    quote! { #(#stmts)* }
}

fn generate_auto_fill_update(ir: &openerp_ir::PersistentIR) -> TokenStream {
    let mut stmts = Vec::new();
    for field in &ir.fields {
        let fname = format_ident!("{}", field.name);
        if let Some(AutoFill::UpdateTimestamp) = &field.auto {
            if field.ty == FieldType::String {
                stmts.push(quote! {
                    record.#fname = chrono::Utc::now().to_rfc3339();
                });
            } else if field.ty == FieldType::Option(Box::new(FieldType::String)) {
                stmts.push(quote! {
                    record.#fname = Some(chrono::Utc::now().to_rfc3339());
                });
            }
        }
    }
    quote! { #(#stmts)* }
}

fn generate_unique_checks(ir: &openerp_ir::PersistentIR) -> TokenStream {
    let mut stmts = Vec::new();
    for idx in &ir.indexes {
        if idx.kind == IndexKind::Unique && idx.fields.len() == 1 {
            let field_name = &idx.fields[0];
            let fname = format_ident!("{}", field_name);
            let idx_prefix = format!("idx:{}:{}:", to_snake_case(&ir.model), field_name);
            let err_msg = format!("{} with this {} already exists", ir.model, field_name);
            stmts.push(quote! {
                {
                    let idx_key = format!("{}{}", #idx_prefix, record.#fname);
                    if self.kv.get(&idx_key).map_err(Self::kv_err)?.is_some() {
                        return Err(openerp_core::ServiceError::Validation(#err_msg.to_string()));
                    }
                }
            });
        }
    }
    quote! { #(#stmts)* }
}

fn generate_unique_index_updates(ir: &openerp_ir::PersistentIR) -> TokenStream {
    let mut stmts = Vec::new();
    for idx in &ir.indexes {
        if idx.kind == IndexKind::Unique && idx.fields.len() == 1 {
            let field_name = &idx.fields[0];
            let fname = format_ident!("{}", field_name);
            let idx_prefix = format!("idx:{}:{}:", to_snake_case(&ir.model), field_name);
            let key_field = format_ident!("{}", &ir.key.fields[0]);
            stmts.push(quote! {
                {
                    let idx_key = format!("{}{}", #idx_prefix, record.#fname);
                    let id_bytes = record.#key_field.to_string().into_bytes();
                    self.kv.set(&idx_key, &id_bytes).map_err(Self::kv_err)?;
                }
            });
        }
    }
    quote! { #(#stmts)* }
}

fn generate_unique_index_deletes(ir: &openerp_ir::PersistentIR) -> TokenStream {
    let mut stmts = Vec::new();
    for idx in &ir.indexes {
        if idx.kind == IndexKind::Unique && idx.fields.len() == 1 {
            let field_name = &idx.fields[0];
            let fname = format_ident!("{}", field_name);
            let idx_prefix = format!("idx:{}:{}:", to_snake_case(&ir.model), field_name);
            stmts.push(quote! {
                {
                    let idx_key = format!("{}{}", #idx_prefix, record.#fname);
                    let _ = self.kv.delete(&idx_key);
                }
            });
        }
    }
    quote! { #(#stmts)* }
}

fn generate_filter_index_updates(ir: &openerp_ir::PersistentIR) -> TokenStream {
    // Filter indexes: store a set of IDs per filter value.
    // For now, use KV with prefix scan for filtering.
    // More sophisticated indexing can be added later.
    quote! {}
}

fn generate_filter_index_deletes(ir: &openerp_ir::PersistentIR) -> TokenStream {
    quote! {}
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
