/// React list component generator

use crate::ir::*;
use anyhow::Result;

pub struct ReactListGenerator;

impl crate::Codegen for ReactListGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let code = generate_list_component(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("components/{}List.tsx", resource.name),
                content: code,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "react-list"
    }
}

fn generate_list_component(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    let component_name = format!("{}List", resource.name);
    
    output.push_str("// Auto-generated list component\n");
    output.push_str("import React, { useState, useEffect } from 'react';\n");
    output.push_str(&format!("import {{ {} }} from '../types';\n", resource.name));
    output.push_str(&format!("import {{ {}Client }} from '../client';\n\n", resource.name));
    
    output.push_str(&format!("export function {}() {{\n", component_name));
    output.push_str(&format!("  const [items, setItems] = useState<{}[]>([]);\n", resource.name));
    output.push_str("  const [loading, setLoading] = useState(true);\n\n");
    
    output.push_str("  useEffect(() => {\n");
    output.push_str("    loadData();\n");
    output.push_str("  }, []);\n\n");
    
    output.push_str("  const loadData = async () => {\n");
    output.push_str("    setLoading(true);\n");
    output.push_str("    try {\n");
    output.push_str(&format!("      const client = new {}Client();\n", resource.name));
    output.push_str("      const data = await client.list();\n");
    output.push_str("      setItems(data);\n");
    output.push_str("    } catch (error) {\n");
    output.push_str("      console.error('Failed to load data:', error);\n");
    output.push_str("    } finally {\n");
    output.push_str("      setLoading(false);\n");
    output.push_str("    }\n");
    output.push_str("  };\n\n");
    
    output.push_str("  if (loading) return <div>Loading...</div>;\n\n");
    
    output.push_str("  return (\n");
    output.push_str(&format!("    <div className=\"{}-list\">\n", to_kebab_case(&resource.name)));
    output.push_str(&format!("      <h1>{}</h1>\n", resource.config.display_name));
    output.push_str("      <table>\n");
    output.push_str("        <thead>\n");
    output.push_str("          <tr>\n");
    
    // Generate table headers
    for field in resource.config.list_columns.iter().take(5) {
        if let Some(f) = resource.fields.iter().find(|f| &f.name == field) {
            let label = f.attrs.ui_label.as_ref().unwrap_or(&f.name);
            output.push_str(&format!("            <th>{}</th>\n", label));
        }
    }
    
    output.push_str("          </tr>\n");
    output.push_str("        </thead>\n");
    output.push_str("        <tbody>\n");
    output.push_str("          {items.map((item) => (\n");
    output.push_str("            <tr key={item.id}>\n");
    
    // Generate table cells
    for field_name in resource.config.list_columns.iter().take(5) {
        let camel_name = to_camel_case(field_name);
        output.push_str(&format!("              <td>{{item.{}}}</td>\n", camel_name));
    }
    
    output.push_str("            </tr>\n");
    output.push_str("          ))}\n");
    output.push_str("        </tbody>\n");
    output.push_str("      </table>\n");
    output.push_str("    </div>\n");
    output.push_str("  );\n");
    output.push_str("}\n");
    
    Ok(output)
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    
    for ch in s.chars() {
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
