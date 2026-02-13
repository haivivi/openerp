//! Parser for `#[facet]` definitions.
//!
//! Reads a struct annotated with `#[facet(path = "/data", auth = "jwt")]`
//! and produces a `FacetIR`.
//!
//! Example:
//! ```ignore
//! #[facet(path = "/data", auth = "jwt", model = "User")]
//! pub struct DataUser {
//!     pub id: String,
//!     pub name: String,
//!     pub email: Option<String>,
//! }
//! ```

use openerp_ir::{AuthMethod, FacetFieldDef, FacetIR, MethodSig};
use syn::{Fields, ItemStruct};

use crate::util;

/// Parse a `#[facet(...)]` annotated struct into `FacetIR`.
pub fn parse_facet(item: &ItemStruct) -> syn::Result<FacetIR> {
    let struct_name = item.ident.to_string();

    let mut facet_name = String::new();
    let mut path = String::new();
    let mut auth = AuthMethod::default();
    let mut model_name = String::new();
    let mut crud = true;

    for attr in &item.attrs {
        if util::attr_is(attr, "facet") {
            attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    let key = ident.to_string();
                    match key.as_str() {
                        "path" => {
                            let v = meta.value()?;
                            let lit: syn::Lit = v.parse()?;
                            if let syn::Lit::Str(s) = lit {
                                path = s.value();
                                // Derive facet name from path: "/data" -> "data"
                                facet_name = path.trim_start_matches('/').to_string();
                            }
                        }
                        "auth" => {
                            let v = meta.value()?;
                            let lit: syn::Lit = v.parse()?;
                            if let syn::Lit::Str(s) = lit {
                                auth = match s.value().as_str() {
                                    "jwt" => AuthMethod::Jwt,
                                    "device_token" => AuthMethod::DeviceToken,
                                    "api_key" => AuthMethod::ApiKey,
                                    "none" => AuthMethod::None,
                                    other => AuthMethod::Custom(other.to_string()),
                                };
                            }
                        }
                        "model" => {
                            let v = meta.value()?;
                            let lit: syn::Lit = v.parse()?;
                            if let syn::Lit::Str(s) = lit {
                                model_name = s.value();
                            }
                        }
                        "crud" => {
                            let v = meta.value()?;
                            let lit: syn::Lit = v.parse()?;
                            if let syn::Lit::Bool(b) = lit {
                                crud = b.value;
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            })?;
        }
    }

    if path.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "facet requires path: #[facet(path = \"/data\")]",
        ));
    }
    if model_name.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "facet requires model: #[facet(model = \"User\")]",
        ));
    }

    // Parse fields.
    let fields = parse_facet_fields(&item.fields)?;

    Ok(FacetIR {
        facet: facet_name,
        path,
        auth,
        model: model_name,
        fields,
        methods: vec![], // Methods parsed separately from impl blocks.
        crud,
    })
}

/// Parse facet struct fields.
fn parse_facet_fields(fields: &Fields) -> syn::Result<Vec<FacetFieldDef>> {
    let named = match fields {
        Fields::Named(named) => named,
        _ => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "facet struct must have named fields",
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
        let readonly = field.attrs.iter().any(|a| util::attr_is(a, "readonly"));

        result.push(FacetFieldDef { name, ty, readonly });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openerp_ir::FieldType;

    fn parse(input: &str) -> FacetIR {
        let item: ItemStruct = syn::parse_str(input).expect("failed to parse");
        parse_facet(&item).expect("failed to parse facet")
    }

    #[test]
    fn data_facet() {
        let f = parse(
            r#"
            #[facet(path = "/data", auth = "jwt", model = "User")]
            pub struct DataUser {
                #[readonly]
                pub id: String,
                pub name: String,
                pub email: Option<String>,
                #[readonly]
                pub created_at: String,
            }
            "#,
        );

        assert_eq!(f.facet, "data");
        assert_eq!(f.path, "/data");
        assert_eq!(f.auth, AuthMethod::Jwt);
        assert_eq!(f.model, "User");
        assert_eq!(f.fields.len(), 4);
        assert!(f.fields[0].readonly); // id
        assert!(!f.fields[1].readonly); // name
        assert!(f.fields[3].readonly); // created_at
        assert!(f.crud);
    }

    #[test]
    fn gear_facet_device_token() {
        let f = parse(
            r#"
            #[facet(path = "/gear", auth = "device_token", model = "Device", crud = false)]
            pub struct GearDevice {
                pub sn: String,
                pub status: DeviceStatus,
            }
            "#,
        );

        assert_eq!(f.facet, "gear");
        assert_eq!(f.auth, AuthMethod::DeviceToken);
        assert!(!f.crud);
        assert_eq!(f.fields.len(), 2);
    }

    #[test]
    fn missing_path_error() {
        let item: ItemStruct = syn::parse_str(
            r#"
            #[facet(model = "User")]
            pub struct Bad { pub id: String }
            "#,
        )
        .unwrap();
        assert!(parse_facet(&item).is_err());
    }

    #[test]
    fn missing_model_error() {
        let item: ItemStruct = syn::parse_str(
            r#"
            #[facet(path = "/data")]
            pub struct Bad { pub id: String }
            "#,
        )
        .unwrap();
        assert!(parse_facet(&item).is_err());
    }

    #[test]
    fn permissions_generated() {
        let f = parse(
            r#"
            #[facet(path = "/data", auth = "jwt", model = "User")]
            pub struct DataUser {
                pub id: String,
                pub name: String,
            }
            "#,
        );

        let perms = f.permissions("auth");
        assert_eq!(perms.len(), 5);
        assert!(perms.contains(&"auth:user:create".to_string()));
    }
}
