/// Codegen Library - shared code generation logic
/// 
/// Used by generated codegen binaries to produce target code from IR metadata.

pub mod sql;
pub mod rust_model;
pub mod rust_service;
pub mod rust_api;
pub mod typescript_types;
pub mod typescript_client;
pub mod react_list;
pub mod react_form;
pub mod react_detail;

use serde::{Deserialize, Serialize};

// Re-export IR types (shared with macro)
pub use crate::ir::*;

pub mod ir {
    use super::*;
    
    /// Complete API schema
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Schema {
        pub resources: Vec<Resource>,
        pub enums: Vec<EnumDef>,
        pub structs: Vec<StructDef>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Resource {
        pub name: String,
        pub fields: Vec<Field>,
        pub config: ResourceConfig,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Field {
        pub name: String,
        pub ty: String,
        pub is_option: bool,
        pub is_vec: bool,
        pub attrs: FieldAttrs,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FieldAttrs {
        pub is_primary_key: bool,
        pub is_required: bool,
        pub is_unique: bool,
        pub is_indexed: bool,
        pub ui_label: Option<String>,
        pub ui_input_type: Option<String>,
        pub ui_placeholder: Option<String>,
        pub relation: Option<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ResourceConfig {
        pub table_name: String,
        pub display_name: String,
        pub list_columns: Vec<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EnumDef {
        pub name: String,
        pub variants: Vec<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StructDef {
        pub name: String,
        pub fields: Vec<Field>,
    }
}

/// Codegen trait - implement this for each target language
pub trait Codegen {
    fn generate(&self, schema: &Schema) -> anyhow::Result<GeneratedCode>;
    fn language(&self) -> &str;
}

pub struct GeneratedCode {
    pub files: Vec<GeneratedFile>,
}

pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}
