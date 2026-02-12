/// Rust service layer generator (CRUD + queries)

use crate::ir::*;
use anyhow::Result;

pub struct RustServiceGenerator;

impl crate::Codegen for RustServiceGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let code = generate_service(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("service/{}.rs", to_snake_case(&resource.name)),
                content: code,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "rust-service"
    }
}

fn generate_service(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    let service_name = format!("{}Service", resource.name);
    let table_name = &resource.config.table_name;
    
    output.push_str("// Auto-generated service layer\n");
    output.push_str("use openerp_core::ServiceError;\n");
    output.push_str("use openerp_sql::SQLStore;\n");
    output.push_str("use std::sync::Arc;\n");
    output.push_str(&format!("use super::model::{};\n\n", resource.name));
    
    // Service struct
    output.push_str(&format!("pub struct {} {{\n", service_name));
    output.push_str("    db: Arc<dyn SQLStore>,\n");
    output.push_str("}\n\n");
    
    // Implementation
    output.push_str(&format!("impl {} {{\n", service_name));
    output.push_str("    pub fn new(db: Arc<dyn SQLStore>) -> Self {\n");
    output.push_str("        Self { db }\n");
    output.push_str("    }\n\n");
    
    // Create
    output.push_str(&format!("    pub async fn create(&self, data: Create{}Request) -> Result<{}, ServiceError> {{\n", resource.name, resource.name));
    output.push_str(&format!("        // TODO: Insert into {}\n", table_name));
    output.push_str("        todo!()\n");
    output.push_str("    }\n\n");
    
    // Get by ID
    output.push_str(&format!("    pub async fn get(&self, id: &str) -> Result<{}, ServiceError> {{\n", resource.name));
    output.push_str(&format!("        // TODO: SELECT FROM {} WHERE id = $1\n", table_name));
    output.push_str("        todo!()\n");
    output.push_str("    }\n\n");
    
    // List with filters
    output.push_str(&format!("    pub async fn list(&self, params: List{}Params) -> Result<Vec<{}>, ServiceError> {{\n", resource.name, resource.name));
    output.push_str(&format!("        // TODO: SELECT FROM {} with filters, sort, pagination\n", table_name));
    output.push_str("        todo!()\n");
    output.push_str("    }\n\n");
    
    // Update
    output.push_str(&format!("    pub async fn update(&self, id: &str, data: Update{}Request) -> Result<{}, ServiceError> {{\n", resource.name, resource.name));
    output.push_str(&format!("        // TODO: UPDATE {} SET ... WHERE id = $1\n", table_name));
    output.push_str("        todo!()\n");
    output.push_str("    }\n\n");
    
    // Delete
    output.push_str("    pub async fn delete(&self, id: &str) -> Result<(), ServiceError> {\n");
    output.push_str(&format!("        // TODO: DELETE FROM {} WHERE id = $1\n", table_name));
    output.push_str("        todo!()\n");
    output.push_str("    }\n");
    
    output.push_str("}\n\n");
    
    // Request/Response types
    output.push_str(&format!("#[derive(Debug, serde::Deserialize)]\n"));
    output.push_str(&format!("pub struct Create{}Request {{\n", resource.name));
    for field in &resource.fields {
        if !field.attrs.is_primary_key {
            output.push_str(&format!("    pub {}: {},\n", 
                to_snake_case(&field.name),
                field.ty
            ));
        }
    }
    output.push_str("}\n\n");
    
    output.push_str(&format!("#[derive(Debug, serde::Deserialize)]\n"));
    output.push_str(&format!("pub struct Update{}Request {{\n", resource.name));
    for field in &resource.fields {
        if !field.attrs.is_primary_key {
            let ty = if field.is_option {
                field.ty.clone()
            } else {
                format!("Option<{}>", field.ty)
            };
            output.push_str(&format!("    pub {}: {},\n", 
                to_snake_case(&field.name),
                ty
            ));
        }
    }
    output.push_str("}\n\n");
    
    output.push_str(&format!("#[derive(Debug, serde::Deserialize)]\n"));
    output.push_str(&format!("pub struct List{}Params {{\n", resource.name));
    output.push_str("    pub _limit: Option<i64>,\n");
    output.push_str("    pub _offset: Option<i64>,\n");
    output.push_str("    pub _sort: Option<String>,\n");
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
