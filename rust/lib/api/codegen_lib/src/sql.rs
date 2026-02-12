/// SQL migration generator

use crate::ir::*;
use anyhow::Result;

pub struct SqlGenerator;

impl crate::Codegen for SqlGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let sql = generate_create_table(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("migrations/001_create_{}.sql", resource.config.table_name),
                content: sql,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "sql"
    }
}

fn generate_create_table(resource: &Resource) -> Result<String> {
    let mut sql = String::new();
    
    sql.push_str(&format!("-- Auto-generated migration for {}\n", resource.name));
    sql.push_str(&format!("CREATE TABLE {} (\n", resource.config.table_name));
    
    // Generate columns
    for (i, field) in resource.fields.iter().enumerate() {
        let col_type = map_rust_type_to_sql(&field.ty, field.is_option);
        let constraints = generate_constraints(field);
        
        sql.push_str(&format!("    {} {}{}", 
            to_snake_case(&field.name),
            col_type,
            constraints
        ));
        
        if i < resource.fields.len() - 1 {
            sql.push_str(",\n");
        } else {
            sql.push_str("\n");
        }
    }
    
    sql.push_str(");\n\n");
    
    // Generate indexes
    for field in &resource.fields {
        if field.attrs.is_indexed || field.attrs.is_unique {
            let idx_type = if field.attrs.is_unique { "UNIQUE " } else { "" };
            sql.push_str(&format!(
                "CREATE {}INDEX idx_{}_{} ON {}({});\n",
                idx_type,
                resource.config.table_name,
                to_snake_case(&field.name),
                resource.config.table_name,
                to_snake_case(&field.name)
            ));
        }
    }
    
    Ok(sql)
}

fn map_rust_type_to_sql(ty: &str, is_optional: bool) -> String {
    let base_type = if ty.starts_with("Option<") {
        &ty[7..ty.len()-1]
    } else {
        ty
    };
    
    let sql_type = match base_type {
        "String" => "TEXT",
        "i32" => "INTEGER",
        "i64" => "BIGINT",
        "f64" => "DOUBLE PRECISION",
        "bool" => "BOOLEAN",
        "DateTime<Utc>" | "DateTime" => "TIMESTAMP WITH TIME ZONE",
        _ if base_type.starts_with("Vec<") => "TEXT[]",  // PostgreSQL array
        _ => "TEXT",  // Default for custom types (store as JSON)
    };
    
    let nullable = if is_optional { "" } else { " NOT NULL" };
    
    format!("{}{}", sql_type, nullable)
}

fn generate_constraints(field: &Field) -> String {
    let mut constraints = String::new();
    
    if field.attrs.is_primary_key {
        constraints.push_str(" PRIMARY KEY");
    }
    
    if field.attrs.is_unique && !field.attrs.is_primary_key {
        constraints.push_str(" UNIQUE");
    }
    
    if let Some(default) = &field.attrs.ui_placeholder {
        // Note: This is a simplified default handling
        // Real implementation would parse default_value attribute
    }
    
    constraints
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
