//! Parser for `#[persistent]` definitions.
//!
//! Reads a struct annotated with `#[persistent(ModelName, store = "kv")]`
//! and produces a `PersistentIR`.

use openerp_ir::{AutoFill, DbFieldDef, IndexDef, IndexKind, KeyDef, PersistentIR, StoreType};
use syn::{Fields, ItemStruct};

use crate::util;

/// Parse a `#[persistent(...)]` annotated struct into `PersistentIR`.
///
/// Expected attributes on the struct:
///   `#[persistent(User, store = "kv")]`
///   `#[key(id)]` or `#[key(field1, field2)]`
///   `#[unique(email)]`
///   `#[index(name)]`
///   `#[search(name, description)]`
///   `#[filter(status)]`
///
/// Expected attributes on fields:
///   `#[auto(create_timestamp)]`
///   `#[auto(update_timestamp)]`
///   `#[auto(uuid)]`
pub fn parse_persistent(item: &ItemStruct) -> syn::Result<PersistentIR> {
    let mut model_name = String::new();
    let mut store = StoreType::default();
    let mut key_fields: Vec<String> = Vec::new();
    let mut indexes: Vec<IndexDef> = Vec::new();

    for attr in &item.attrs {
        if util::attr_is(attr, "persistent") {
            // Parse #[persistent(ModelName, store = "kv")]
            let mut first_ident = true;
            attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    let key = ident.to_string();
                    if key == "store" {
                        let v = meta.value()?;
                        let lit: syn::Lit = v.parse()?;
                        if let syn::Lit::Str(s) = lit {
                            store = match s.value().as_str() {
                                "kv" => StoreType::Kv,
                                "sql" => StoreType::Sql,
                                other => {
                                    return Err(meta.error(format!(
                                        "unknown store type: '{}', expected 'kv' or 'sql'",
                                        other
                                    )))
                                }
                            };
                        }
                    } else if first_ident {
                        model_name = key;
                        first_ident = false;
                    }
                }
                Ok(())
            })?;
        } else if util::attr_is(attr, "key") {
            key_fields = util::parse_ident_list(attr)?;
        } else if util::attr_is(attr, "unique") {
            let fields = util::parse_ident_list(attr)?;
            indexes.push(IndexDef {
                fields,
                kind: IndexKind::Unique,
            });
        } else if util::attr_is(attr, "index") {
            let fields = util::parse_ident_list(attr)?;
            indexes.push(IndexDef {
                fields,
                kind: IndexKind::Index,
            });
        } else if util::attr_is(attr, "search") {
            let fields = util::parse_ident_list(attr)?;
            indexes.push(IndexDef {
                fields,
                kind: IndexKind::Search,
            });
        } else if util::attr_is(attr, "filter") {
            let fields = util::parse_ident_list(attr)?;
            indexes.push(IndexDef {
                fields,
                kind: IndexKind::Filter,
            });
        }
    }

    if model_name.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "persistent requires model name: #[persistent(ModelName)]",
        ));
    }

    if key_fields.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "persistent requires key: #[key(field_name)]",
        ));
    }

    // Parse fields.
    let fields = parse_db_fields(&item.fields, &model_name)?;

    // Validate key fields exist.
    for kf in &key_fields {
        if !fields.iter().any(|f| &f.name == kf) {
            return Err(syn::Error::new_spanned(
                &item.ident,
                format!("key field '{}' not found in DB struct fields", kf),
            ));
        }
    }

    // Validate index fields exist.
    for idx in &indexes {
        for f in &idx.fields {
            if !fields.iter().any(|field| &field.name == f) {
                return Err(syn::Error::new_spanned(
                    &item.ident,
                    format!("index field '{}' not found in DB struct fields", f),
                ));
            }
        }
    }

    let key = if key_fields.len() == 1 {
        KeyDef::single(key_fields.into_iter().next().unwrap())
    } else {
        KeyDef::compound(key_fields)
    };

    Ok(PersistentIR {
        model: model_name,
        store,
        key,
        indexes,
        fields,
    })
}

/// Parse DB struct fields. Fields not present in the source model are marked hidden.
/// (The hidden check is done by the validator later, not here.)
fn parse_db_fields(fields: &Fields, _model: &str) -> syn::Result<Vec<DbFieldDef>> {
    let named = match fields {
        Fields::Named(named) => named,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "persistent struct must have named fields",
            ))
        }
    };

    let mut result = Vec::new();
    for field in &named.named {
        let name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "field must have a name"))?
            .to_string();

        let ty = util::parse_field_type(&field.ty);
        let auto = parse_auto_attr(&field.attrs)?;

        result.push(DbFieldDef {
            name,
            ty,
            hidden: false, // Will be set by validator comparing with model fields.
            auto,
        });
    }
    Ok(result)
}

/// Parse `#[auto(...)]` attribute on a field.
fn parse_auto_attr(attrs: &[syn::Attribute]) -> syn::Result<Option<AutoFill>> {
    for attr in attrs {
        if util::attr_is(attr, "auto") {
            let idents = util::parse_ident_list(attr)?;
            if let Some(kind) = idents.first() {
                return Ok(Some(match kind.as_str() {
                    "create_timestamp" => AutoFill::CreateTimestamp,
                    "update_timestamp" => AutoFill::UpdateTimestamp,
                    "uuid" => AutoFill::Uuid,
                    other => {
                        return Err(syn::Error::new_spanned(
                            attr,
                            format!(
                                "unknown auto kind: '{}', expected create_timestamp/update_timestamp/uuid",
                                other
                            ),
                        ))
                    }
                }));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openerp_ir::FieldType;

    fn parse(input: &str) -> PersistentIR {
        let item: ItemStruct = syn::parse_str(input).expect("failed to parse");
        parse_persistent(&item).expect("failed to parse persistent")
    }

    #[test]
    fn simple_persistent() {
        let db = parse(
            r#"
            #[persistent(User, store = "kv")]
            #[key(id)]
            #[unique(email)]
            #[index(name)]
            pub struct UserDB {
                #[auto(uuid)]
                pub id: String,
                pub name: String,
                pub email: Option<String>,
                pub password_hash: String,
                #[auto(create_timestamp)]
                pub created_at: String,
                #[auto(update_timestamp)]
                pub updated_at: String,
            }
            "#,
        );

        assert_eq!(db.model, "User");
        assert_eq!(db.store, StoreType::Kv);
        assert_eq!(db.key.fields, vec!["id"]);
        assert_eq!(db.fields.len(), 6);
        assert_eq!(db.indexes.len(), 2);
        assert_eq!(db.indexes[0].kind, IndexKind::Unique);
        assert_eq!(db.indexes[0].fields, vec!["email"]);
        assert_eq!(db.indexes[1].kind, IndexKind::Index);

        // Auto-fill fields.
        let auto_fields = db.auto_fields();
        assert_eq!(auto_fields.len(), 3);
    }

    #[test]
    fn compound_key_persistent() {
        let db = parse(
            r#"
            #[persistent(Firmware, store = "kv")]
            #[key(model, semver)]
            pub struct FirmwareDB {
                pub model: u32,
                pub semver: String,
                pub build: u64,
            }
            "#,
        );

        assert!(db.key.is_compound());
        assert_eq!(db.key.fields, vec!["model", "semver"]);
    }

    #[test]
    fn search_and_filter_indexes() {
        let db = parse(
            r#"
            #[persistent(Device, store = "kv")]
            #[key(sn)]
            #[search(sn, description)]
            #[filter(status, model)]
            pub struct DeviceDB {
                pub sn: String,
                pub model: u32,
                pub status: DeviceStatus,
                pub description: Option<String>,
            }
            "#,
        );

        assert_eq!(db.indexes.len(), 2);
        assert_eq!(db.indexes_of(IndexKind::Search).len(), 1);
        assert_eq!(db.indexes_of(IndexKind::Filter).len(), 1);
        assert_eq!(
            db.indexes_of(IndexKind::Search)[0].fields,
            vec!["sn", "description"]
        );
    }

    #[test]
    fn missing_model_name() {
        let item: ItemStruct = syn::parse_str(
            r#"
            #[persistent()]
            #[key(id)]
            pub struct Bad { pub id: String }
            "#,
        )
        .unwrap();
        assert!(parse_persistent(&item).is_err());
    }
}
