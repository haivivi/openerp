//! Parser for `#[model]` definitions.
//!
//! Reads a struct annotated with `#[model(module = "auth")]` and `#[key(id)]`
//! and produces a `ModelIR`.

use openerp_ir::{FieldDef, FieldRef, KeyDef, ModelIR, UiWidget};
use syn::{Fields, ItemStruct};

use crate::util;

/// Parse a `#[model(...)]` annotated struct into `ModelIR`.
///
/// Expected attributes on the struct:
///   `#[model(module = "auth")]`
///   `#[key(id)]` or `#[key(model, semver)]` for compound keys
///
/// Expected attributes on fields:
///   `#[ref(ModelName)]` — foreign key reference
///   `#[serde(rename = "type")]` — serde rename
pub fn parse_model(item: &ItemStruct) -> syn::Result<ModelIR> {
    let name = item.ident.to_string();

    // Parse struct-level attributes.
    let mut module = String::new();
    let mut key_fields: Vec<String> = Vec::new();

    for attr in &item.attrs {
        if util::attr_is(attr, "model") {
            let kvs = util::parse_kv_attrs(attr)?;
            for (k, v) in kvs {
                if k == "module" {
                    module = v;
                }
            }
        } else if util::attr_is(attr, "key") {
            key_fields = util::parse_ident_list(attr)?;
        }
    }

    if module.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "model requires module attribute: #[model(module = \"...\")]",
        ));
    }

    if key_fields.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "model requires key attribute: #[key(field_name)]",
        ));
    }

    // Parse fields.
    let fields = parse_struct_fields(&item.fields)?;

    // Validate key fields exist.
    for kf in &key_fields {
        if !fields.iter().any(|f| &f.name == kf) {
            return Err(syn::Error::new_spanned(
                &item.ident,
                format!("key field '{}' not found in struct fields", kf),
            ));
        }
    }

    let key = if key_fields.len() == 1 {
        KeyDef::single(key_fields.into_iter().next().unwrap())
    } else {
        KeyDef::compound(key_fields)
    };

    // Extract doc comment.
    let doc = extract_doc_comment(&item.attrs);

    Ok(ModelIR {
        name,
        module,
        key,
        fields,
        methods: vec![], // Methods are parsed separately from impl blocks or fn items.
        doc,
    })
}

/// Parse struct fields into `FieldDef` list.
fn parse_struct_fields(fields: &Fields) -> syn::Result<Vec<FieldDef>> {
    let named = match fields {
        Fields::Named(named) => named,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "model struct must have named fields",
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
        let reference = parse_field_ref(&field.attrs)?;
        let serde_rename = util::extract_serde_rename(&field.attrs);
        let doc = extract_doc_comment(&field.attrs);
        let ui_widget = parse_ui_widget(&field.attrs, &ty, &name)?;

        result.push(FieldDef {
            name,
            ty,
            ui_widget,
            doc,
            reference,
            serde_rename,
        });
    }
    Ok(result)
}

/// Determine the UI widget for a field.
///
/// Priority:
/// 1. Explicit `#[ui(widget = "switch")]` attribute
/// 2. Well-known field name heuristics (email, avatar, password, etc.)
/// 3. Default from Rust type (bool -> Switch, Vec<String> -> Tags, etc.)
fn parse_ui_widget(
    attrs: &[syn::Attribute],
    ty: &openerp_ir::FieldType,
    field_name: &str,
) -> syn::Result<Option<UiWidget>> {
    // 1. Check for explicit #[ui(widget = "...")] attribute.
    for attr in attrs {
        if crate::util::attr_is(attr, "ui") {
            let kvs = crate::util::parse_kv_attrs(attr)?;
            for (k, v) in kvs {
                if k == "widget" {
                    let widget = match v.as_str() {
                        "text" => UiWidget::Text,
                        "textarea" => UiWidget::Textarea,
                        "number" => UiWidget::Number,
                        "switch" => UiWidget::Switch,
                        "checkbox" => UiWidget::Checkbox,
                        "select" => UiWidget::Select,
                        "tags" => UiWidget::Tags,
                        "email" => UiWidget::Email,
                        "url" => UiWidget::Url,
                        "password" => UiWidget::Password,
                        "image_upload" => UiWidget::ImageUpload,
                        "file_upload" => UiWidget::FileUpload,
                        "date" => UiWidget::Date,
                        "datetime" => UiWidget::DateTime,
                        "color" => UiWidget::Color,
                        "markdown" => UiWidget::Markdown,
                        "code" => UiWidget::Code,
                        "hidden" => UiWidget::Hidden,
                        "readonly" => UiWidget::ReadOnly,
                        "permission_picker" => UiWidget::PermissionPicker,
                        other => {
                            return Err(syn::Error::new_spanned(
                                attr,
                                format!("unknown ui widget: '{}'", other),
                            ))
                        }
                    };
                    return Ok(Some(widget));
                }
            }
        }
    }

    // 2. Well-known field name heuristics.
    let name_lower = field_name.to_lowercase();
    let by_name = match name_lower.as_str() {
        "email" => Some(UiWidget::Email),
        "avatar" | "avatar_url" | "image" | "photo" => Some(UiWidget::ImageUpload),
        "password" | "password_hash" => Some(UiWidget::Password),
        "description" | "notes" | "bio" | "content" | "release_notes" => Some(UiWidget::Textarea),
        "url" | "link" | "redirect_url" | "auth_url" | "token_url" | "userinfo_url" => {
            Some(UiWidget::Url)
        }
        "color" | "hex_color" => Some(UiWidget::Color),
        _ if name_lower.ends_with("_url") => Some(UiWidget::Url),
        _ if name_lower.ends_with("_at") => Some(UiWidget::DateTime),
        _ => None,
    };
    if let Some(w) = by_name {
        return Ok(Some(w));
    }

    // 3. Check if the type name is a well-known newtype.
    match ty {
        openerp_ir::FieldType::Enum(name) | openerp_ir::FieldType::Struct(name) => {
            if let Some(w) = UiWidget::from_type_name(name) {
                return Ok(Some(w));
            }
        }
        openerp_ir::FieldType::Option(inner) => {
            if let openerp_ir::FieldType::Enum(name) | openerp_ir::FieldType::Struct(name) =
                inner.as_ref()
            {
                if let Some(w) = UiWidget::from_type_name(name) {
                    return Ok(Some(w));
                }
            }
        }
        _ => {}
    }

    // 4. Default from Rust type — only emit for non-text types (text is the implicit default).
    let default_widget = UiWidget::from_field_type(ty);
    if default_widget != UiWidget::Text {
        return Ok(Some(default_widget));
    }

    // Return None = frontend uses Text as default.
    Ok(None)
}

/// Parse `#[ref(ModelName)]` or `#[ref(ModelName, get = "...", list = "...", ...)]`
fn parse_field_ref(attrs: &[syn::Attribute]) -> syn::Result<Option<FieldRef>> {
    for attr in attrs {
        if util::attr_is(attr, "r#ref") || attr.path().is_ident("ref") {
            // Try to parse as a list of key-value pairs first.
            let mut target = String::new();
            let mut get = None;
            let mut list = None;
            let mut value = None;
            let mut display = None;

            attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    let key = ident.to_string();
                    // First bare ident is the target model name.
                    if target.is_empty()
                        && !["get", "list", "value", "display"].contains(&key.as_str())
                    {
                        target = key;
                    } else if key == "get" {
                        let v = meta.value()?;
                        let lit: syn::Lit = v.parse()?;
                        if let syn::Lit::Str(s) = lit {
                            get = Some(s.value());
                        }
                    } else if key == "list" {
                        let v = meta.value()?;
                        let lit: syn::Lit = v.parse()?;
                        if let syn::Lit::Str(s) = lit {
                            list = Some(s.value());
                        }
                    } else if key == "value" {
                        let v = meta.value()?;
                        let lit: syn::Lit = v.parse()?;
                        if let syn::Lit::Str(s) = lit {
                            value = Some(s.value());
                        }
                    } else if key == "display" {
                        let v = meta.value()?;
                        let lit: syn::Lit = v.parse()?;
                        if let syn::Lit::Str(s) = lit {
                            display = Some(s.value());
                        }
                    }
                }
                Ok(())
            })?;

            if !target.is_empty() {
                return Ok(Some(FieldRef {
                    target,
                    get,
                    list,
                    value,
                    display,
                }));
            }
        }
    }
    Ok(None)
}

/// Extract `///` doc comments from attributes.
fn extract_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    return util::attr_string_value(nv);
                }
            }
            None
        })
        .map(|s| s.trim().to_string())
        .collect();

    if docs.is_empty() {
        None
    } else {
        Some(docs.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openerp_ir::FieldType;

    fn parse(input: &str) -> ModelIR {
        let item: ItemStruct = syn::parse_str(input).expect("failed to parse");
        parse_model(&item).expect("failed to parse model")
    }

    #[test]
    fn simple_model() {
        let model = parse(
            r#"
            #[model(module = "auth")]
            #[key(id)]
            pub struct User {
                pub id: String,
                pub name: String,
                pub email: Option<String>,
                pub created_at: String,
            }
            "#,
        );

        assert_eq!(model.name, "User");
        assert_eq!(model.module, "auth");
        assert_eq!(model.key.fields, vec!["id"]);
        assert_eq!(model.fields.len(), 4);
        assert_eq!(model.fields[0].name, "id");
        assert_eq!(model.fields[0].ty, FieldType::String);
        assert_eq!(model.fields[2].name, "email");
        assert!(model.fields[2].ty.is_optional());
    }

    #[test]
    fn compound_key() {
        let model = parse(
            r#"
            #[model(module = "pms")]
            #[key(model, semver)]
            pub struct Firmware {
                pub model: u32,
                pub semver: String,
                pub build: u64,
            }
            "#,
        );

        assert!(model.key.is_compound());
        assert_eq!(model.key.fields, vec!["model", "semver"]);
    }

    #[test]
    fn field_types() {
        let model = parse(
            r#"
            #[model(module = "pms")]
            #[key(sn)]
            pub struct Device {
                pub sn: String,
                pub model: u32,
                pub status: DeviceStatus,
                pub imei: Vec<String>,
                pub licenses: Vec<String>,
                pub description: Option<String>,
            }
            "#,
        );

        assert_eq!(model.fields[1].ty, FieldType::U32);
        assert_eq!(model.fields[2].ty, FieldType::Enum("DeviceStatus".into()));
        assert!(model.fields[3].ty.is_vec());
        assert!(model.fields[5].ty.is_optional());
    }

    #[test]
    fn missing_module_error() {
        let item: ItemStruct = syn::parse_str(
            r#"
            #[model()]
            #[key(id)]
            pub struct Bad {
                pub id: String,
            }
            "#,
        )
        .unwrap();
        assert!(parse_model(&item).is_err());
    }

    #[test]
    fn missing_key_error() {
        let item: ItemStruct = syn::parse_str(
            r#"
            #[model(module = "test")]
            pub struct Bad {
                pub id: String,
            }
            "#,
        )
        .unwrap();
        assert!(parse_model(&item).is_err());
    }

    #[test]
    fn invalid_key_field_error() {
        let item: ItemStruct = syn::parse_str(
            r#"
            #[model(module = "test")]
            #[key(nonexistent)]
            pub struct Bad {
                pub id: String,
            }
            "#,
        )
        .unwrap();
        assert!(parse_model(&item).is_err());
    }

    #[test]
    fn permissions_generated() {
        let model = parse(
            r#"
            #[model(module = "pms")]
            #[key(code)]
            pub struct Model {
                pub code: u32,
                pub series_name: String,
            }
            "#,
        );

        let perms = model.crud_permissions();
        assert!(perms.contains(&"pms:model:create".to_string()));
        assert!(perms.contains(&"pms:model:read".to_string()));
        assert!(perms.contains(&"pms:model:list".to_string()));
    }

    #[test]
    fn serde_roundtrip() {
        let model = parse(
            r#"
            #[model(module = "auth")]
            #[key(id)]
            pub struct User {
                pub id: String,
                pub name: String,
            }
            "#,
        );
        let json = serde_json::to_string(&model).unwrap();
        let back: ModelIR = serde_json::from_str(&json).unwrap();
        assert_eq!(model.name, back.name);
        assert_eq!(model.fields.len(), back.fields.len());
    }
}
