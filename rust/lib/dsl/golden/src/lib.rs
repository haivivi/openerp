//! Golden tests for the DSL framework.
//!
//! Verify that DSL definitions produce exactly the expected output.
//! If the macro or schema builder changes, these tests break and show the diff.

#[cfg(test)]
mod tests {
    use openerp_macro::model;
    use openerp_store::{
        build_schema, HierarchyNode, KvOps, KvStore, ModuleDef, ResourceDef,
        apply_overrides,
    };
    use openerp_types::*;
    use std::sync::Arc;

    // =====================================================================
    // Sample DSL definitions
    // =====================================================================

    #[model(module = "test")]
    pub struct Widget {
        pub id: Id,
        pub count: u32,
        pub active: bool,
        pub email: Option<Email>,
        pub tags: Vec<String>,
        pub password_hash: Option<PasswordHash>,
    }

    impl KvStore for Widget {
        const KEY: Field = Self::id;
        fn kv_prefix() -> &'static str { "test:widget:" }
        fn key_value(&self) -> String { self.id.to_string() }
        fn before_create(&mut self) {
            if self.id.is_empty() {
                self.id = Id::new("auto-id");
            }
            self.created_at = DateTime::new("2026-01-01T00:00:00Z");
            self.updated_at = DateTime::new("2026-01-01T00:00:00Z");
        }
    }

    #[model(module = "test")]
    pub struct Item {
        pub id: Id,
        pub widget_id: Id,
        pub quantity: u32,
        #[ui(widget = "select")]
        pub status: String,
    }

    // =====================================================================
    // Golden: Model IR structure
    // =====================================================================

    #[test]
    fn golden_widget_ir_structure() {
        let ir = Widget::__dsl_ir();
        assert_eq!(ir["name"], "Widget");
        assert_eq!(ir["module"], "test");
        assert_eq!(ir["resource"], "widget");

        let fields = ir["fields"].as_array().unwrap();
        let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();

        // User-defined fields.
        assert!(names.contains(&"id"));
        assert!(names.contains(&"count"));
        assert!(names.contains(&"active"));
        assert!(names.contains(&"email"));
        assert!(names.contains(&"tags"));
        assert!(names.contains(&"password_hash"));

        // Auto-injected common fields.
        assert!(names.contains(&"display_name"), "missing display_name in {:?}", names);
        assert!(names.contains(&"description"), "missing description in {:?}", names);
        assert!(names.contains(&"metadata"), "missing metadata in {:?}", names);
        assert!(names.contains(&"created_at"), "missing created_at in {:?}", names);
        assert!(names.contains(&"updated_at"), "missing updated_at in {:?}", names);

        // Total: 6 user + 5 common = 11.
        assert_eq!(fields.len(), 11, "expected 11 fields, got: {:?}", names);
    }

    #[test]
    fn golden_item_ir_with_ui_override() {
        let ir = Item::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let status = fields.iter().find(|f| f["name"] == "status").unwrap();
        assert_eq!(status["widget"], "select", "explicit #[ui(widget)] override");
    }

    // =====================================================================
    // Golden: Widget inference by type
    // =====================================================================

    #[test]
    fn golden_widget_type_inference() {
        assert_eq!(Widget::id.widget, "readonly");        // Id -> readonly
        assert_eq!(Widget::count.widget, "text");            // u32 -> text (not a known newtype)
        assert_eq!(Widget::active.widget, "switch");       // bool -> switch
        assert_eq!(Widget::email.widget, "email");         // Email -> email
        assert_eq!(Widget::tags.widget, "tags");            // Vec<String> -> tags
        assert_eq!(Widget::password_hash.widget, "hidden"); // PasswordHash -> hidden
        assert_eq!(Widget::display_name.widget, "text");   // String (common)
        assert_eq!(Widget::description.widget, "textarea"); // "description" heuristic
        assert_eq!(Widget::created_at.widget, "datetime"); // DateTime (common)
        assert_eq!(Widget::updated_at.widget, "datetime"); // DateTime (common)
    }

    // =====================================================================
    // Golden: ResourceDef auto-derivation
    // =====================================================================

    #[test]
    fn golden_resource_def_from_ir() {
        let def = ResourceDef::from_ir("test", Widget::__dsl_ir());
        assert_eq!(def.name, "widget");
        assert_eq!(def.label, "Widgets");
        assert_eq!(def.path, "widgets");
        assert_eq!(def.icon, "file"); // default icon for unknown resource
        assert_eq!(def.permissions.len(), 5);
        assert!(def.permissions.contains(&"test:widget:create".to_string()));
        assert!(def.permissions.contains(&"test:widget:read".to_string()));
        assert!(def.permissions.contains(&"test:widget:update".to_string()));
        assert!(def.permissions.contains(&"test:widget:delete".to_string()));
        assert!(def.permissions.contains(&"test:widget:list".to_string()));
    }

    #[test]
    fn golden_resource_def_with_action() {
        let def = ResourceDef::from_ir("test", Item::__dsl_ir())
            .with_action("test", "approve")
            .with_action("test", "reject");
        assert_eq!(def.permissions.len(), 7); // 5 CRUD + approve + reject
        assert!(def.permissions.contains(&"test:item:approve".to_string()));
        assert!(def.permissions.contains(&"test:item:reject".to_string()));
    }

    // =====================================================================
    // Golden: Schema builder output
    // =====================================================================

    #[test]
    fn golden_schema_structure() {
        let schema = build_schema(
            "TestApp",
            vec![ModuleDef {
                id: "test",
                label: "Test Module",
                icon: "flask",
                resources: vec![
                    ResourceDef::from_ir("test", Widget::__dsl_ir()).with_desc("Test widgets"),
                    ResourceDef::from_ir("test", Item::__dsl_ir()).with_desc("Widget items"),
                ],
                hierarchy: vec![
                    HierarchyNode {
                        resource: "widget", label: "Widgets", icon: "cube",
                        description: "Test widgets",
                        children: vec![
                            HierarchyNode::leaf("item", "Items", "file-text", "Widget items"),
                        ],
                    },
                ],
            }],
        );

        assert_eq!(schema["name"], "TestApp");
        assert_eq!(schema["modules"].as_array().unwrap().len(), 1);

        let m = &schema["modules"][0];
        assert_eq!(m["id"], "test");
        assert_eq!(m["resources"].as_array().unwrap().len(), 2);

        // Hierarchy tree.
        let nav = m["hierarchy"]["nav"].as_array().unwrap();
        assert_eq!(nav.len(), 1); // Only Widget at top level.
        assert_eq!(nav[0]["resource"], "widget");
        assert_eq!(nav[0]["children"].as_array().unwrap().len(), 1);
        assert_eq!(nav[0]["children"][0]["resource"], "item");

        // Permissions.
        let perms = &schema["permissions"]["test"];
        assert!(perms["widget"]["actions"].as_array().unwrap().len() >= 5);
        assert!(perms["item"]["actions"].as_array().unwrap().len() >= 5);

        // Permission entry structure.
        let first_perm = &perms["widget"]["actions"][0];
        assert!(first_perm["perm"].is_string());
        assert!(first_perm["action"].is_string());
        assert!(first_perm["desc"].is_string());
    }

    #[test]
    fn golden_schema_ui_overrides() {
        let mut schema = build_schema(
            "TestApp",
            vec![ModuleDef {
                id: "test",
                label: "Test",
                icon: "flask",
                resources: vec![
                    ResourceDef::from_ir("test", Item::__dsl_ir()),
                ],
                hierarchy: vec![HierarchyNode::leaf("item", "Items", "file-text", "Items")],
            }],
        );

        let overrides = vec![
            openerp_store::widget!(select {
                source: "/admin/test/widgets",
                display: "display_name",
                value: "id"
            } => [Item.widget_id]),
        ];
        apply_overrides(&mut schema, &overrides);

        // Check the override was applied.
        let fields = schema["modules"][0]["resources"][0]["fields"].as_array().unwrap();
        let wid_field = fields.iter().find(|f| f["name"] == "widget_id").unwrap();
        assert_eq!(wid_field["widget"], "select");
        assert_eq!(wid_field["source"], "/admin/test/widgets");
        assert_eq!(wid_field["display"], "display_name");
        assert_eq!(wid_field["value"], "id");
    }

    // =====================================================================
    // Golden: Hierarchy to_json
    // =====================================================================

    #[test]
    fn golden_hierarchy_json() {
        let tree = HierarchyNode {
            resource: "model", label: "Models", icon: "cube",
            description: "Product models",
            children: vec![
                HierarchyNode::leaf("device", "Devices", "desktop", "Produced devices"),
                HierarchyNode::leaf("batch", "Batches", "package", "Production batches"),
            ],
        };

        let json = tree.to_json();
        assert_eq!(json["resource"], "model");
        assert_eq!(json["label"], "Models");
        assert_eq!(json["children"].as_array().unwrap().len(), 2);
        assert_eq!(json["children"][0]["resource"], "device");
        assert_eq!(json["children"][1]["resource"], "batch");
        // Leaf children have empty children array.
        assert_eq!(json["children"][0]["children"].as_array().unwrap().len(), 0);
    }

    // =====================================================================
    // Golden: KvStore CRUD lifecycle
    // =====================================================================

    #[test]
    fn golden_kvstore_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let kv: Arc<dyn openerp_kv::KVStore> =
            Arc::new(openerp_kv::RedbStore::open(&dir.path().join("golden.redb")).unwrap());
        let ops = KvOps::<Widget>::new(kv);

        // Create with hook.
        let w = Widget {
            id: Id::default(),
            count: 42,
            active: true,
            email: Some(Email::new("test@test.com")),
            tags: vec!["a".into(), "b".into()],
            password_hash: None,
            display_name: Some("Test".into()),
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
        };
        let created = ops.save_new(w).unwrap();
        assert_eq!(created.id.as_str(), "auto-id");
        assert_eq!(created.created_at.as_str(), "2026-01-01T00:00:00Z");

        // Get.
        let fetched = ops.get_or_err("auto-id").unwrap();
        assert_eq!(fetched.count, 42);

        // List.
        assert_eq!(ops.list().unwrap().len(), 1);

        // Update.
        let mut u = fetched;
        u.count = 100;
        ops.save(u).unwrap();
        assert_eq!(ops.get_or_err("auto-id").unwrap().count, 100);

        // Delete.
        ops.delete("auto-id").unwrap();
        assert!(ops.get("auto-id").unwrap().is_none());
        assert_eq!(ops.list().unwrap().len(), 0);
    }

    // =====================================================================
    // Golden: Serde camelCase + defaults
    // =====================================================================

    #[test]
    fn golden_serde_camel_case() {
        let w = Widget {
            id: Id::new("x"),
            count: 1,
            active: true,
            email: None,
            tags: vec![],
            password_hash: None,
            display_name: None,
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
        };
        let json = serde_json::to_value(&w).unwrap();
        // camelCase keys.
        assert!(json.get("displayName").is_some(), "expected camelCase displayName");
        assert!(json.get("passwordHash").is_some(), "expected camelCase passwordHash");
        assert!(json.get("createdAt").is_some(), "expected camelCase createdAt");
    }

    #[test]
    fn golden_serde_defaults() {
        // Deserialize with missing fields — all should default.
        let json = r#"{"count": 5}"#;
        let w: Widget = serde_json::from_str(json).unwrap();
        assert_eq!(w.count, 5);
        assert!(w.id.is_empty()); // default Id
        assert!(!w.active); // default bool = false
        assert!(w.email.is_none()); // default Option
        assert!(w.tags.is_empty()); // default Vec
        assert!(w.display_name.is_none()); // common field default
    }
}
