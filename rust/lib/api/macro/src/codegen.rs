/// Code generator - generates codegen binary source from IR

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::parse::ResourceIR;

/// Generate a codegen binary that contains the resource metadata
pub fn generate_resource_codegen(
    original: &DeriveInput,
    resource: &ResourceIR,
) -> TokenStream {
    let original_name = &original.ident;
    let resource_json = serde_json::to_string(&resource).unwrap();
    let codegen_binary_name = format!("{}_codegen", original_name.to_string().to_lowercase());
    
    // 生成：
    // 1. 原始 struct（保持不变）
    // 2. 一个包含元信息的常量
    // 3. 一个 codegen binary（在单独的 crate 中）
    
    quote! {
        // 保留原始 struct
        #original
        
        // 生成元信息常量（在编译时嵌入）
        #[doc(hidden)]
        pub const __RESOURCE_METADATA: &str = #resource_json;
        
        // 注册到全局 codegen registry（由 macro 收集所有资源）
        #[doc(hidden)]
        #[linkme::distributed_slice(RESOURCE_REGISTRY)]
        static __RESOURCE_ENTRY: &str = #resource_json;
    }
}

pub fn generate_enum_codegen(
    original: &DeriveInput,
    _enum_def: &str,
) -> TokenStream {
    quote! {
        #original
    }
}

pub fn generate_action_codegen(
    original: &DeriveInput,
    _action: &str,
) -> TokenStream {
    quote! {
        #original
    }
}

pub fn generate_filter_codegen(
    original: &DeriveInput,
    _filters: &str,
) -> TokenStream {
    quote! {
        #original
    }
}
