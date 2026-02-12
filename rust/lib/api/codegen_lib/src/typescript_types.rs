/// TypeScript types generator

use crate::ir::*;
use anyhow::Result;

pub struct TypeScriptTypesGenerator;

impl crate::Codegen for TypeScriptTypesGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        // Generate index file that exports all types
        let mut index_content = String::from("// Auto-generated TypeScript types\n\n");
        
        for resource in &schema.resources {
            let type_code = generate_interface(resource)?;
            index_content.push_str(&type_code);
            index_content.push_str("\n");
        }
        
        files.push(crate::GeneratedFile {
            path: "types/index.ts".to_string(),
            content: index_content,
        });
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "typescript-types"
    }
}

fn generate_interface(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    
    output.push_str(&format!("export interface {} {{\n", resource.name));
    
    for field in &resource.fields {
        let ts_type = map_rust_type_to_ts(&field.ty, field.is_option);
        let optional = if field.is_option { "?" } else { "" };
        
        if let Some(label) = &field.attrs.ui_label {
            output.push_str(&format!("  /** {} */\n", label));
        }
        
        output.push_str(&format!("  {}{}: {};\n", 
            to_camel_case(&field.name),
            optional,
            ts_type
        ));
    }
    
    output.push_str("}\n");
    
    Ok(output)
}

fn map_rust_type_to_ts(ty: &str, is_optional: bool) -> String {
    let base_type = if ty.starts_with("Option<") {
        &ty[7..ty.len()-1]
    } else {
        ty
    };
    
    let ts_type = match base_type {
        "String" => "string",
        "i32" | "i64" | "f64" => "number",
        "bool" => "boolean",
        "DateTime<Utc>" | "DateTime" => "string",  // ISO 8601
        _ if base_type.starts_with("Vec<") => {
            let inner = &base_type[4..base_type.len()-1];
            return format!("{}[]", map_rust_type_to_ts(inner, false));
        }
        _ => base_type,  // Custom types
    };
    
    ts_type.to_string()
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    
    for (i, ch) in s.chars().enumerate() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    
    result
}
