/// openerp-codegen-macro
/// 
/// Proc macros for full-stack code generation from Rust DSL.
/// 
/// Usage:
/// ```ignore
/// #[resource(table = "characters", display_name = "角色")]
/// pub struct Character {
///     #[primary_key]
///     pub id: String,
///     
///     #[required]
///     #[ui(label = "名称", input_type = "text")]
///     pub name: String,
/// }
/// ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

mod parse;
mod codegen;

/// Derive macro for resource definitions
/// 
/// Generates:
/// 1. Embedded IR metadata
/// 2. Codegen binary that uses the metadata
#[proc_macro_attribute]
pub fn resource(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    
    // Parse resource configuration from attributes
    let resource = match parse::parse_resource(&input, &attr) {
        Ok(r) => r,
        Err(e) => return e.to_compile_error().into(),
    };
    
    // Generate:
    // 1. Original struct (unchanged)
    // 2. Codegen binary with embedded metadata
    let output = codegen::generate_resource_codegen(&input, &resource);
    
    output.into()
}

/// Derive macro for enum definitions
#[proc_macro_derive(Enum, attributes(ui))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let enum_def = match parse::parse_enum(&input) {
        Ok(e) => e,
        Err(e) => return e.to_compile_error().into(),
    };
    
    let output = codegen::generate_enum_codegen(&input, &enum_def);
    
    output.into()
}

/// Attribute macro for custom actions
#[proc_macro_attribute]
pub fn action(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    
    let action = match parse::parse_action(&input, &attr) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };
    
    let output = codegen::generate_action_codegen(&input, &action);
    
    output.into()
}

/// Attribute macro for filter definitions
#[proc_macro_attribute]
pub fn filters(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    
    // Parse which resource this filter applies to
    let resource_name = match parse::parse_filter_target(&attr) {
        Ok(name) => name,
        Err(e) => return e.to_compile_error().into(),
    };
    
    let filters = match parse::parse_filters(&input, &resource_name) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error().into(),
    };
    
    let output = codegen::generate_filter_codegen(&input, &filters);
    
    output.into()
}
