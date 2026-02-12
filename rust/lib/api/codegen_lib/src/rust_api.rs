/// Rust REST API handlers generator

use crate::ir::*;
use anyhow::Result;

pub struct RustApiGenerator;

impl crate::Codegen for RustApiGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let code = generate_handlers(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("api/{}.rs", to_snake_case(&resource.name)),
                content: code,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "rust-api"
    }
}

fn generate_handlers(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    let service_name = format!("{}Service", resource.name);
    let base_path = format!("/{}", to_kebab_case(&resource.config.table_name));
    
    output.push_str("// Auto-generated REST API handlers\n");
    output.push_str("use axum::{\n");
    output.push_str("    extract::{Path, Query, State},\n");
    output.push_str("    http::StatusCode,\n");
    output.push_str("    response::Json,\n");
    output.push_str("    routing::{get, post, patch, delete},\n");
    output.push_str("    Router,\n");
    output.push_str("};\n");
    output.push_str("use std::sync::Arc;\n");
    output.push_str(&format!("use super::service::{{{}Service, *}};\n", resource.name));
    output.push_str(&format!("use super::model::{};\n\n", resource.name));
    
    // Router function
    output.push_str(&format!("pub fn {}_routes(service: Arc<{}>) -> Router {{\n", 
        to_snake_case(&resource.name),
        service_name
    ));
    output.push_str("    Router::new()\n");
    output.push_str(&format!("        .route(\"{}\", post(create_handler).get(list_handler))\n", base_path));
    output.push_str(&format!("        .route(\"{}/:id\", get(get_handler).patch(update_handler).delete(delete_handler))\n", base_path));
    output.push_str("        .with_state(service)\n");
    output.push_str("}\n\n");
    
    // Create handler
    output.push_str("async fn create_handler(\n");
    output.push_str(&format!("    State(svc): State<Arc<{}>>,\n", service_name));
    output.push_str(&format!("    Json(body): Json<Create{}Request>,\n", resource.name));
    output.push_str(&format!(") -> Result<Json<{}>, StatusCode> {{\n", resource.name));
    output.push_str("    let result = svc.create(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;\n");
    output.push_str("    Ok(Json(result))\n");
    output.push_str("}\n\n");
    
    // Get handler
    output.push_str("async fn get_handler(\n");
    output.push_str(&format!("    State(svc): State<Arc<{}>>,\n", service_name));
    output.push_str("    Path(id): Path<String>,\n");
    output.push_str(&format!(") -> Result<Json<{}>, StatusCode> {{\n", resource.name));
    output.push_str("    let result = svc.get(&id).await.map_err(|_| StatusCode::NOT_FOUND)?;\n");
    output.push_str("    Ok(Json(result))\n");
    output.push_str("}\n\n");
    
    // List handler
    output.push_str("async fn list_handler(\n");
    output.push_str(&format!("    State(svc): State<Arc<{}>>,\n", service_name));
    output.push_str(&format!("    Query(params): Query<List{}Params>,\n", resource.name));
    output.push_str(&format!(") -> Result<Json<Vec<{}>>, StatusCode> {{\n", resource.name));
    output.push_str("    let result = svc.list(params).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;\n");
    output.push_str("    Ok(Json(result))\n");
    output.push_str("}\n\n");
    
    // Update handler
    output.push_str("async fn update_handler(\n");
    output.push_str(&format!("    State(svc): State<Arc<{}>>,\n", service_name));
    output.push_str("    Path(id): Path<String>,\n");
    output.push_str(&format!("    Json(body): Json<Update{}Request>,\n", resource.name));
    output.push_str(&format!(") -> Result<Json<{}>, StatusCode> {{\n", resource.name));
    output.push_str("    let result = svc.update(&id, body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;\n");
    output.push_str("    Ok(Json(result))\n");
    output.push_str("}\n\n");
    
    // Delete handler
    output.push_str("async fn delete_handler(\n");
    output.push_str(&format!("    State(svc): State<Arc<{}>>,\n", service_name));
    output.push_str("    Path(id): Path<String>,\n");
    output.push_str(") -> Result<StatusCode, StatusCode> {\n");
    output.push_str("    svc.delete(&id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;\n");
    output.push_str("    Ok(StatusCode::NO_CONTENT)\n");
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

fn to_kebab_case(s: &str) -> String {
    to_snake_case(s).replace('_', "-")
}
