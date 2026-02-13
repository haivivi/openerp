//! Shared parser utilities.

use syn::{Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue};

/// Extract a string literal from a name-value attribute.
/// e.g. `#[something(key = "value")]` → `"value"` for key.
pub fn attr_string_value(nv: &MetaNameValue) -> Option<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(s), ..
    }) = &nv.value
    {
        Some(s.value())
    } else {
        None
    }
}

/// Check if an attribute has a given path identifier.
pub fn attr_is(attr: &Attribute, name: &str) -> bool {
    attr.path().is_ident(name)
}

/// Parse a parenthesized list of identifiers from an attribute.
/// e.g. `#[key(id, name)]` → `["id", "name"]`
pub fn parse_ident_list(attr: &Attribute) -> syn::Result<Vec<String>> {
    let mut result = Vec::new();
    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            result.push(ident.to_string());
        }
        Ok(())
    })?;
    Ok(result)
}

/// Parse key-value pairs from a parenthesized attribute.
/// e.g. `#[model(module = "auth")]` → `[("module", "auth")]`
pub fn parse_kv_attrs(attr: &Attribute) -> syn::Result<Vec<(String, String)>> {
    let mut result = Vec::new();
    attr.parse_nested_meta(|meta| {
        if let Some(ident) = meta.path.get_ident() {
            let key = ident.to_string();
            let value = meta.value()?;
            let lit: Lit = value.parse()?;
            if let Lit::Str(s) = lit {
                result.push((key, s.value()));
            }
        }
        Ok(())
    })?;
    Ok(result)
}

/// Convert CamelCase to snake_case.
pub fn to_snake_case(s: &str) -> String {
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

/// Parse a Rust type into an IR FieldType.
pub fn parse_field_type(ty: &syn::Type) -> openerp_ir::FieldType {
    use openerp_ir::FieldType;

    match ty {
        syn::Type::Path(tp) => {
            let seg = &tp.path.segments;
            if seg.len() == 1 {
                let name = seg[0].ident.to_string();
                match name.as_str() {
                    "String" => FieldType::String,
                    "bool" => FieldType::Bool,
                    "u32" => FieldType::U32,
                    "u64" => FieldType::U64,
                    "i32" => FieldType::I32,
                    "i64" => FieldType::I64,
                    "f64" => FieldType::F64,
                    "Option" => {
                        if let syn::PathArguments::AngleBracketed(args) = &seg[0].arguments {
                            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                return FieldType::Option(Box::new(parse_field_type(inner)));
                            }
                        }
                        FieldType::Option(Box::new(FieldType::String))
                    }
                    "Vec" => {
                        if let syn::PathArguments::AngleBracketed(args) = &seg[0].arguments {
                            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                                return FieldType::Vec(Box::new(parse_field_type(inner)));
                            }
                        }
                        FieldType::Vec(Box::new(FieldType::String))
                    }
                    // Everything else is a named type (enum or struct).
                    other => FieldType::Enum(other.to_string()),
                }
            } else {
                // Multi-segment path, treat as custom type.
                let full: String = seg
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                FieldType::Struct(full)
            }
        }
        _ => FieldType::Json, // Fallback for complex types.
    }
}

/// Extract serde rename from field attributes.
/// e.g. `#[serde(rename = "type")]` → Some("type")
pub fn extract_serde_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            if let Meta::List(list) = &attr.meta {
                let tokens = list.tokens.to_string();
                // Simple parse: look for rename = "..."
                if let Some(pos) = tokens.find("rename") {
                    let rest = &tokens[pos..];
                    if let Some(start) = rest.find('"') {
                        let rest = &rest[start + 1..];
                        if let Some(end_quote) = rest.find('"') {
                            return Some(rest[..end_quote].to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
