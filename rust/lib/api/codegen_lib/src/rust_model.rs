/// Rust model generator

use crate::ir::*;
use anyhow::Result;

pub struct RustModelGenerator;

impl crate::Codegen for RustModelGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let code = generate_model_struct(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("model/{}.rs", to_snake_case(&resource.name)),
                content: code,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "rust-model"
    }
}

fn generate_model_struct(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    
    output.push_str("// Auto-generated model\n");
    output.push_str("use serde::{Deserialize, Serialize};\n");
    output.push_str("use chrono::{DateTime, Utc};\n\n");
    
    output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    output.push_str("#[serde(rename_all = \"camelCase\")]\n");
    output.push_str(&format!("pub struct {} {{\n", resource.name));
    
    for field in &resource.fields {
        if let Some(label) = &field.attrs.ui_label {
            output.push_str(&format!("    /// {}\n", label));
        }
        
        output.push_str(&format!("    pub {}: {},\n", 
            to_snake_case(&field.name),
            field.ty
        ));
    }
    
    output.push_str("}\n");
    
    Ok(output)
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !result.ends_with('_') {
                result.push('_');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}
