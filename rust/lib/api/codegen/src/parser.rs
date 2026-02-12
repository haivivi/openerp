use anyhow::{Result, bail};
use crate::ir::*;

/// Parse .api file into IR
pub fn parse(input: &str) -> Result<Schema> {
    let mut types = Vec::new();
    let mut service = None;
    
    let mut lines = input.lines().peekable();
    
    while let Some(line) = lines.next() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        
        // Parse struct
        if line.starts_with("struct ") {
            let type_def = parse_struct(line, &mut lines)?;
            types.push(type_def);
        }
        
        // Parse service
        if line.starts_with("service ") {
            service = Some(parse_service(line, &mut lines)?);
        }
    }
    
    let service = service.ok_or_else(|| anyhow::anyhow!("No service definition found"))?;
    
    Ok(Schema { types, service })
}

fn parse_struct(first_line: &str, lines: &mut std::iter::Peekable<std::str::Lines>) -> Result<TypeDef> {
    let name = first_line
        .strip_prefix("struct ")
        .and_then(|s| s.strip_suffix(" {"))
        .ok_or_else(|| anyhow::anyhow!("Invalid struct syntax"))?
        .trim()
        .to_string();
    
    let mut fields = Vec::new();
    
    while let Some(line) = lines.next() {
        let line = line.trim();
        
        if line == "}" {
            break;
        }
        
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        
        // Parse field: name: Type,
        if let Some(stripped) = line.strip_suffix(",") {
            let parts: Vec<&str> = stripped.split(": ").collect();
            if parts.len() == 2 {
                let field_name = parts[0].trim().to_string();
                let type_str = parts[1].trim();
                let ty = parse_type(type_str)?;
                fields.push(Field { name: field_name, ty });
            }
        }
    }
    
    Ok(TypeDef { name, fields })
}

fn parse_type(s: &str) -> Result<Type> {
    if s == "String" {
        Ok(Type::String)
    } else if s == "i32" {
        Ok(Type::I32)
    } else if s == "i64" {
        Ok(Type::I64)
    } else if s == "bool" {
        Ok(Type::Bool)
    } else if s == "()" {
        Ok(Type::Custom("()".to_string()))
    } else if let Some(inner) = s.strip_prefix("Option<").and_then(|s| s.strip_suffix(">")) {
        Ok(Type::Option(Box::new(parse_type(inner)?)))
    } else if let Some(inner) = s.strip_prefix("Vec<").and_then(|s| s.strip_suffix(">")) {
        Ok(Type::Vec(Box::new(parse_type(inner)?)))
    } else {
        Ok(Type::Custom(s.to_string()))
    }
}

fn parse_service(first_line: &str, lines: &mut std::iter::Peekable<std::str::Lines>) -> Result<Service> {
    let name = first_line
        .strip_prefix("service ")
        .and_then(|s| s.strip_suffix(" {"))
        .ok_or_else(|| anyhow::anyhow!("Invalid service syntax"))?
        .trim()
        .to_string();
    
    let mut endpoints = Vec::new();
    let mut current_doc: Option<String> = None;
    
    while let Some(line) = lines.next() {
        let line = line.trim();
        
        if line == "}" {
            break;
        }
        
        if line.is_empty() {
            continue;
        }
        
        // Parse doc comment
        if let Some(doc) = line.strip_prefix("///") {
            current_doc = Some(doc.trim().to_string());
            continue;
        }
        
        // Parse endpoint: GET "/path" name(params) -> ReturnType;
        if line.starts_with("GET ") || line.starts_with("POST ") || line.starts_with("PUT ") 
            || line.starts_with("PATCH ") || line.starts_with("DELETE ") {
            let endpoint = parse_endpoint(line, current_doc.take())?;
            endpoints.push(endpoint);
        }
    }
    
    Ok(Service { name, endpoints })
}

fn parse_endpoint(line: &str, doc: Option<String>) -> Result<Endpoint> {
    // Example: POST "/users" create_user(body: CreateUserRequest) -> User;
    let line = line.strip_suffix(";").unwrap_or(line);
    
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() != 2 {
        bail!("Invalid endpoint syntax");
    }
    
    let method = match parts[0] {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        _ => bail!("Unknown HTTP method"),
    };
    
    let rest = parts[1];
    
    // Parse path
    let path_start = rest.find('"').ok_or_else(|| anyhow::anyhow!("Missing path"))?;
    let path_end = rest[path_start + 1..].find('"').ok_or_else(|| anyhow::anyhow!("Missing path end"))?;
    let path = rest[path_start + 1..path_start + 1 + path_end].to_string();
    
    // Parse name and params
    let after_path = &rest[path_end + 1..].trim();
    let name_start = after_path.find(char::is_alphabetic).unwrap_or(0);
    let params_start = after_path.find('(').ok_or_else(|| anyhow::anyhow!("Missing params"))?;
    let name = after_path[name_start..params_start].trim().to_string();
    
    let params_end = after_path.find(')').ok_or_else(|| anyhow::anyhow!("Missing )"))?;
    let params_str = &after_path[params_start + 1..params_end];
    let params = parse_params(params_str)?;
    
    // Parse return type
    let return_str = after_path[params_end + 1..].trim();
    let return_type = if let Some(ret) = return_str.strip_prefix("->").map(|s| s.trim()) {
        parse_type(ret)?
    } else {
        Type::Custom("()".to_string())
    };
    
    Ok(Endpoint {
        method,
        path,
        name,
        params,
        return_type,
        doc,
    })
}

fn parse_params(s: &str) -> Result<Vec<Param>> {
    if s.trim().is_empty() {
        return Ok(Vec::new());
    }
    
    let mut params = Vec::new();
    
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        
        // Format: "kind name: Type" or "kind name: Type = default"
        // Example: "body: CreateUserRequest" or "path id: String" or "query page: i32 = 1"
        
        // Split by colon first
        let colon_parts: Vec<&str> = part.splitn(2, ':').collect();
        if colon_parts.len() != 2 {
            continue;
        }
        
        let left = colon_parts[0].trim();
        let right = colon_parts[1].trim();
        
        // Parse left side (kind and name)
        let left_tokens: Vec<&str> = left.split_whitespace().collect();
        if left_tokens.is_empty() {
            continue;
        }
        
        let (kind, name) = if left_tokens.len() == 1 {
            // Just "body:" - infer kind from context
            let token = left_tokens[0];
            match token {
                "body" => (ParamKind::Body, "body".to_string()),
                _ => continue,
            }
        } else {
            // "path id:" or "query page:"
            let kind = match left_tokens[0] {
                "path" => ParamKind::Path,
                "query" => ParamKind::Query,
                "body" => ParamKind::Body,
                _ => continue,
            };
            let name = left_tokens[1].to_string();
            (kind, name)
        };
        
        // Parse right side (type and optional default)
        let right_parts: Vec<&str> = right.split('=').map(|s| s.trim()).collect();
        let ty_str = right_parts[0];
        let ty = parse_type(ty_str)?;
        let default = right_parts.get(1).map(|s| s.to_string());
        
        params.push(Param { name, ty, kind, default });
    }
    
    Ok(params)
}
