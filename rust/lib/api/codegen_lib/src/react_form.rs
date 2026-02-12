/// React form component generator

use crate::ir::*;
use anyhow::Result;

pub struct ReactFormGenerator;

impl crate::Codegen for ReactFormGenerator {
    fn generate(&self, schema: &Schema) -> Result<crate::GeneratedCode> {
        let mut files = Vec::new();
        
        for resource in &schema.resources {
            let code = generate_form_component(resource)?;
            files.push(crate::GeneratedFile {
                path: format!("components/{}Form.tsx", resource.name),
                content: code,
            });
        }
        
        Ok(crate::GeneratedCode { files })
    }
    
    fn language(&self) -> &str {
        "react-form"
    }
}

fn generate_form_component(resource: &Resource) -> Result<String> {
    let mut output = String::new();
    let component_name = format!("{}Form", resource.name);
    
    output.push_str("// Auto-generated form component\n");
    output.push_str("import React, { useState } from 'react';\n");
    output.push_str(&format!("import {{ Create{}Request }} from '../types';\n", resource.name));
    output.push_str(&format!("import {{ {}Client }} from '../client';\n\n", resource.name));
    
    output.push_str(&format!("interface {}Props {{\n", component_name));
    output.push_str("  onSuccess?: () => void;\n");
    output.push_str("}\n\n");
    
    output.push_str(&format!("export function {}({{ onSuccess }}: {}Props) {{\n", component_name, component_name));
    
    // Generate state for each field
    output.push_str("  const [formData, setFormData] = useState({\n");
    for field in &resource.fields {
        if !field.attrs.is_primary_key {
            let default_val = if field.is_option { "null" } else { "\"\"" };
            output.push_str(&format!("    {}: {},\n", to_camel_case(&field.name), default_val));
        }
    }
    output.push_str("  });\n\n");
    
    output.push_str("  const handleSubmit = async (e: React.FormEvent) => {\n");
    output.push_str("    e.preventDefault();\n");
    output.push_str("    try {\n");
    output.push_str(&format!("      const client = new {}Client();\n", resource.name));
    output.push_str("      await client.create(formData);\n");
    output.push_str("      onSuccess?.();\n");
    output.push_str("    } catch (error) {\n");
    output.push_str("      console.error('Failed to save:', error);\n");
    output.push_str("    }\n");
    output.push_str("  };\n\n");
    
    output.push_str("  return (\n");
    output.push_str("    <form onSubmit={handleSubmit}>\n");
    
    // Generate form fields
    for field in &resource.fields {
        if field.attrs.is_primary_key {
            continue;
        }
        
        let label = field.attrs.ui_label.as_ref().unwrap_or(&field.name);
        let input_type = field.attrs.ui_input_type.as_deref().unwrap_or("text");
        let field_name = to_camel_case(&field.name);
        
        output.push_str("      <div>\n");
        output.push_str(&format!("        <label>{}</label>\n", label));
        
        if input_type == "textarea" {
            output.push_str(&format!("        <textarea\n"));
            output.push_str(&format!("          value={{formData.{}}}\n", field_name));
            output.push_str(&format!("          onChange={{(e) => setFormData({{...formData, {}: e.target.value}})}}\n", field_name));
            output.push_str("        />\n");
        } else {
            output.push_str(&format!("        <input\n"));
            output.push_str(&format!("          type=\"{}\"\n", input_type));
            output.push_str(&format!("          value={{formData.{}}}\n", field_name));
            output.push_str(&format!("          onChange={{(e) => setFormData({{...formData, {}: e.target.value}})}}\n", field_name));
            output.push_str("        />\n");
        }
        
        output.push_str("      </div>\n");
    }
    
    output.push_str("      <button type=\"submit\">Save</button>\n");
    output.push_str("    </form>\n");
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
