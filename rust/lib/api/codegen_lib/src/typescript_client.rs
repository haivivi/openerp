/// TypeScript client SDK generator

use crate::ir::*;
use anyhow::Result;

pub struct TypeScriptClientGenerator;

impl crate::Codegen for TypeScriptClientGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let code = generate_client(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("client/{}Client.ts", resource.name),
                content: code,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "typescript-client"
    }
}

fn generate_client(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    let client_name = format!("{}Client", resource.name);
    let base_path = format!("/{}", to_kebab_case(&resource.config.table_name));
    
    output.push_str("// Auto-generated client SDK\n");
    output.push_str(&format!("import {{ {}, Create{}Request, Update{}Request, List{}Params }} from '../types';\n\n",
        resource.name, resource.name, resource.name, resource.name));
    
    output.push_str(&format!("export class {} {{\n", client_name));
    output.push_str("  constructor(\n");
    output.push_str("    private baseUrl: string = '/api',\n");
    output.push_str("    private token?: string\n");
    output.push_str("  ) {}\n\n");
    
    output.push_str("  private async fetch<T>(path: string, options?: RequestInit): Promise<T> {\n");
    output.push_str("    const headers = new Headers(options?.headers);\n");
    output.push_str("    headers.set('Content-Type', 'application/json');\n");
    output.push_str("    if (this.token) {\n");
    output.push_str("      headers.set('Authorization', `Bearer ${this.token}`);\n");
    output.push_str("    }\n\n");
    output.push_str("    const response = await fetch(`${this.baseUrl}${path}`, {\n");
    output.push_str("      ...options,\n");
    output.push_str("      headers,\n");
    output.push_str("    });\n\n");
    output.push_str("    if (!response.ok) {\n");
    output.push_str("      throw new Error(`HTTP ${response.status}: ${response.statusText}`);\n");
    output.push_str("    }\n\n");
    output.push_str("    if (response.status === 204) return undefined as any;\n");
    output.push_str("    return response.json();\n");
    output.push_str("  }\n\n");
    
    // Create
    output.push_str(&format!("  async create(data: Create{}Request): Promise<{}> {{\n", resource.name, resource.name));
    output.push_str(&format!("    return this.fetch<{}>('{}', {{\n", resource.name, base_path));
    output.push_str("      method: 'POST',\n");
    output.push_str("      body: JSON.stringify(data),\n");
    output.push_str("    });\n");
    output.push_str("  }\n\n");
    
    // Get
    output.push_str(&format!("  async get(id: string): Promise<{}> {{\n", resource.name));
    output.push_str(&format!("    return this.fetch<{}>(`{}/$${{id}}`);\n", resource.name, base_path));
    output.push_str("  }\n\n");
    
    // List
    output.push_str(&format!("  async list(params?: List{}Params): Promise<{}[]> {{\n", resource.name, resource.name));
    output.push_str("    const query = new URLSearchParams();\n");
    output.push_str("    if (params?._limit) query.set('_limit', params._limit.toString());\n");
    output.push_str("    if (params?._offset) query.set('_offset', params._offset.toString());\n");
    output.push_str("    if (params?._sort) query.set('_sort', params._sort);\n\n");
    output.push_str(&format!("    return this.fetch<{}[]>(`{}?$${{query}}`);\n", resource.name, base_path));
    output.push_str("  }\n\n");
    
    // Update
    output.push_str(&format!("  async update(id: string, data: Update{}Request): Promise<{}> {{\n", resource.name, resource.name));
    output.push_str(&format!("    return this.fetch<{}>(`{}/$${{id}}`, {{\n", resource.name, base_path));
    output.push_str("      method: 'PATCH',\n");
    output.push_str("      body: JSON.stringify(data),\n");
    output.push_str("    });\n");
    output.push_str("  }\n\n");
    
    // Delete
    output.push_str("  async delete(id: string): Promise<void> {\n");
    output.push_str(&format!("    return this.fetch<void>(`{}/$${{id}}`, {{\n", base_path));
    output.push_str("      method: 'DELETE',\n");
    output.push_str("    });\n");
    output.push_str("  }\n");
    
    output.push_str("}\n");
    
    Ok(output)
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}
