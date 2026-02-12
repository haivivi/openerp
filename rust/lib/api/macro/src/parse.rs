/// Parser for extracting IR from Rust structs with attributes

use syn::{DeriveInput, Data, Fields, Field, Attribute, Lit, Meta, NestedMeta};
use quote::ToTokens;

// Import IR types (will be shared between macro and codegen)
// For now, define simplified versions here
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize)]
pub struct ResourceIR {
    pub name: String,
    pub fields: Vec<FieldIR>,
    pub config: ResourceConfigIR,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldIR {
    pub name: String,
    pub ty: String,
    pub is_option: bool,
    pub is_vec: bool,
    pub attrs: FieldAttrsIR,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldAttrsIR {
    pub is_primary_key: bool,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_indexed: bool,
    pub ui_label: Option<String>,
    pub ui_input_type: Option<String>,
    pub ui_placeholder: Option<String>,
    pub relation: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceConfigIR {
    pub table_name: String,
    pub display_name: String,
    pub list_columns: Vec<String>,
}

pub type AttributeArgs = Vec<NestedMeta>;

pub fn parse_resource(
    input: &DeriveInput,
    attrs: &AttributeArgs,
) -> syn::Result<ResourceIR> {
    let name = input.ident.to_string();
    
    // Parse resource-level attributes
    let config = parse_resource_config(&name, attrs)?;
    
    // Parse fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                parse_fields(&fields.named)?
            }
            _ => return Err(syn::Error::new_spanned(input, "Resource must have named fields")),
        },
        _ => return Err(syn::Error::new_spanned(input, "Resource must be a struct")),
    };
    
    Ok(ResourceIR {
        name,
        fields,
        config,
    })
}

fn parse_resource_config(
    name: &str,
    attrs: &AttributeArgs,
) -> syn::Result<ResourceConfigIR> {
    let mut table_name = to_snake_case(name);
    let mut display_name = name.to_string();
    let mut list_columns = Vec::new();
    
    for attr in attrs {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = attr {
            let name = nv.path.get_ident().map(|i| i.to_string());
            
            match name.as_deref() {
                Some("table") => {
                    if let Lit::Str(s) = &nv.lit {
                        table_name = s.value();
                    }
                }
                Some("display_name") => {
                    if let Lit::Str(s) = &nv.lit {
                        display_name = s.value();
                    }
                }
                Some("list_columns") => {
                    // Parse array of strings
                    // Simplified: just use default for now
                }
                _ => {}
            }
        }
    }
    
    Ok(ResourceConfigIR {
        table_name,
        display_name,
        list_columns,
    })
}

fn parse_fields(
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
) -> syn::Result<Vec<FieldIR>> {
    let mut result = Vec::new();
    
    for field in fields {
        let name = field.ident.as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "Field must have a name"))?
            .to_string();
        
        let ty = field.ty.to_token_stream().to_string();
        let is_option = ty.starts_with("Option");
        let is_vec = ty.starts_with("Vec");
        
        let attrs = parse_field_attrs(&field.attrs)?;
        
        result.push(FieldIR {
            name,
            ty,
            is_option,
            is_vec,
            attrs,
        });
    }
    
    Ok(result)
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrsIR> {
    let mut result = FieldAttrsIR {
        is_primary_key: false,
        is_required: false,
        is_unique: false,
        is_indexed: false,
        ui_label: None,
        ui_input_type: None,
        ui_placeholder: None,
        relation: None,
    };
    
    for attr in attrs {
        if attr.path.is_ident("primary_key") {
            result.is_primary_key = true;
        } else if attr.path.is_ident("required") {
            result.is_required = true;
        } else if attr.path.is_ident("unique") {
            result.is_unique = true;
        } else if attr.path.is_ident("index") {
            result.is_indexed = true;
        } else if attr.path.is_ident("ui") {
            parse_ui_attrs(attr, &mut result)?;
        } else if attr.path.is_ident("belongs_to") {
            // Parse belongs_to(TargetType)
            if let Ok(Meta::List(list)) = attr.parse_meta() {
                if let Some(NestedMeta::Meta(Meta::Path(path))) = list.nested.first() {
                    result.relation = Some(path.get_ident().unwrap().to_string());
                }
            }
        }
    }
    
    Ok(result)
}

fn parse_ui_attrs(attr: &Attribute, result: &mut FieldAttrsIR) -> syn::Result<()> {
    if let Ok(Meta::List(list)) = attr.parse_meta() {
        for nested in &list.nested {
            if let NestedMeta::Meta(Meta::NameValue(nv)) = nested {
                let name = nv.path.get_ident().map(|i| i.to_string());
                
                match name.as_deref() {
                    Some("label") => {
                        if let Lit::Str(s) = &nv.lit {
                            result.ui_label = Some(s.value());
                        }
                    }
                    Some("input_type") => {
                        if let Lit::Str(s) = &nv.lit {
                            result.ui_input_type = Some(s.value());
                        }
                    }
                    Some("placeholder") => {
                        if let Lit::Str(s) = &nv.lit {
                            result.ui_placeholder = Some(s.value());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    Ok(())
}

pub fn parse_enum(_input: &DeriveInput) -> syn::Result<String> {
    // Simplified for now
    Ok("EnumDef".to_string())
}

pub fn parse_action(
    _input: &DeriveInput,
    _attrs: &AttributeArgs,
) -> syn::Result<String> {
    Ok("ActionDef".to_string())
}

pub fn parse_filter_target(_attrs: &TokenStream) -> syn::Result<String> {
    Ok("FilterDef".to_string())
}

pub fn parse_filters(
    _input: &DeriveInput,
    _target: &str,
) -> syn::Result<String> {
    Ok("FiltersDef".to_string())
}

// Helper: convert CamelCase to snake_case
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
