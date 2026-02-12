/// Integration test for full-stack codegen

use openerp_codegen_lib::*;

#[test]
fn test_full_stack_codegen() {
    // Create a simple test resource
    let schema = ir::Schema {
        resources: vec![
            ir::Resource {
                name: "User".to_string(),
                fields: vec![
                    ir::Field {
                        name: "id".to_string(),
                        ty: "String".to_string(),
                        is_option: false,
                        is_vec: false,
                        attrs: ir::FieldAttrs {
                            is_primary_key: true,
                            is_required: true,
                            is_unique: false,
                            is_indexed: false,
                            ui_label: Some("ID".to_string()),
                            ui_input_type: None,
                            ui_placeholder: None,
                            relation: None,
                        },
                    },
                    ir::Field {
                        name: "name".to_string(),
                        ty: "String".to_string(),
                        is_option: false,
                        is_vec: false,
                        attrs: ir::FieldAttrs {
                            is_primary_key: false,
                            is_required: true,
                            is_unique: false,
                            is_indexed: false,
                            ui_label: Some("Name".to_string()),
                            ui_input_type: Some("text".to_string()),
                            ui_placeholder: None,
                            relation: None,
                        },
                    },
                    ir::Field {
                        name: "email".to_string(),
                        ty: "String".to_string(),
                        is_option: false,
                        is_vec: false,
                        attrs: ir::FieldAttrs {
                            is_primary_key: false,
                            is_required: true,
                            is_unique: true,
                            is_indexed: false,
                            ui_label: Some("Email".to_string()),
                            ui_input_type: Some("email".to_string()),
                            ui_placeholder: None,
                            relation: None,
                        },
                    },
                ],
                config: ir::ResourceConfig {
                    table_name: "users".to_string(),
                    display_name: "User".to_string(),
                    list_columns: vec!["name".to_string(), "email".to_string()],
                },
            },
        ],
        enums: vec![],
        structs: vec![],
    };
    
    // Test SQL generator
    let sql_gen = sql::SqlGenerator;
    let sql_result = sql_gen.generate(&schema).unwrap();
    assert_eq!(sql_result.files.len(), 1);
    assert!(sql_result.files[0].content.contains("CREATE TABLE users"));
    println!("✅ SQL migration generated");
    
    // Test Rust model generator
    let model_gen = rust_model::RustModelGenerator;
    let model_result = model_gen.generate(&schema).unwrap();
    assert_eq!(model_result.files.len(), 1);
    assert!(model_result.files[0].content.contains("pub struct User"));
    println!("✅ Rust model generated");
    
    // Test Rust service generator
    let service_gen = rust_service::RustServiceGenerator;
    let service_result = service_gen.generate(&schema).unwrap();
    assert_eq!(service_result.files.len(), 1);
    assert!(service_result.files[0].content.contains("pub struct UserService"));
    assert!(service_result.files[0].content.contains("pub async fn create"));
    println!("✅ Rust service generated");
    
    // Test Rust API generator
    let api_gen = rust_api::RustApiGenerator;
    let api_result = api_gen.generate(&schema).unwrap();
    assert_eq!(api_result.files.len(), 1);
    assert!(api_result.files[0].content.contains("pub fn user_routes"));
    assert!(api_result.files[0].content.contains("async fn create_handler"));
    println!("✅ Rust API handlers generated");
    
    // Test TypeScript types generator
    let ts_types_gen = typescript_types::TypeScriptTypesGenerator;
    let ts_types_result = ts_types_gen.generate(&schema).unwrap();
    assert_eq!(ts_types_result.files.len(), 1);
    assert!(ts_types_result.files[0].content.contains("export interface User"));
    println!("✅ TypeScript types generated");
    
    // Test TypeScript client generator
    let ts_client_gen = typescript_client::TypeScriptClientGenerator;
    let ts_client_result = ts_client_gen.generate(&schema).unwrap();
    assert_eq!(ts_client_result.files.len(), 1);
    assert!(ts_client_result.files[0].content.contains("export class UserClient"));
    println!("✅ TypeScript client generated");
    
    // Test React list generator
    let react_list_gen = react_list::ReactListGenerator;
    let react_list_result = react_list_gen.generate(&schema).unwrap();
    assert_eq!(react_list_result.files.len(), 1);
    assert!(react_list_result.files[0].content.contains("export function UserList"));
    println!("✅ React list component generated");
    
    // Test React form generator
    let react_form_gen = react_form::ReactFormGenerator;
    let react_form_result = react_form_gen.generate(&schema).unwrap();
    assert_eq!(react_form_result.files.len(), 1);
    assert!(react_form_result.files[0].content.contains("export function UserForm"));
    println!("✅ React form component generated");
    
    println!("\n🎉 All generators working!");
    println!("\nGenerated {} files:", 
        sql_result.files.len() +
        model_result.files.len() +
        service_result.files.len() +
        api_result.files.len() +
        ts_types_result.files.len() +
        ts_client_result.files.len() +
        react_list_result.files.len() +
        react_form_result.files.len()
    );
    
    for file in &sql_result.files {
        println!("  - {}", file.path);
    }
    for file in &model_result.files {
        println!("  - {}", file.path);
    }
    for file in &service_result.files {
        println!("  - {}", file.path);
    }
    for file in &api_result.files {
        println!("  - {}", file.path);
    }
    for file in &ts_types_result.files {
        println!("  - {}", file.path);
    }
    for file in &ts_client_result.files {
        println!("  - {}", file.path);
    }
    for file in &react_list_result.files {
        println!("  - {}", file.path);
    }
    for file in &react_form_result.files {
        println!("  - {}", file.path);
    }
    
    // Print sample generated code
    println!("\n📝 Sample generated SQL migration:");
    println!("---");
    println!("{}", sql_result.files[0].content.lines().take(10).collect::<Vec<_>>().join("\n"));
    println!("...");
    
    println!("\n📝 Sample generated TypeScript interface:");
    println!("---");
    println!("{}", ts_types_result.files[0].content.lines().take(8).collect::<Vec<_>>().join("\n"));
    println!("...");
}
