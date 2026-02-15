//! Integration tests for the #[model] macro.

#[cfg(test)]
mod tests {
    use openerp_macro::model;
    use openerp_types::*;

    // ── Basic model (common fields auto-injected) ──

    #[model(module = "auth")]
    pub struct User {
        pub id: Id,
        pub email: Option<Email>,
        pub avatar: Option<Avatar>,
        pub active: bool,
        pub password_hash: Option<PasswordHash>,
    }

    #[test]
    fn model_has_serde() {
        let user = User {
            id: Id::new("u1"),
            email: Some(Email::new("alice@test.com")),
            avatar: None,
            active: true,
            password_hash: None,
            // Common fields (auto-injected):
            display_name: Some("Alice".into()),
            description: None,
            metadata: None,
            created_at: DateTime::new("2024-01-01T00:00:00Z"),
            updated_at: DateTime::new("2024-01-01T00:00:00Z"),
            version: 0,
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"passwordHash\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"displayName\""));
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"alice@test.com\""));
    }

    #[test]
    fn common_fields_auto_injected() {
        // These Field consts exist even though not in the struct definition.
        let _: Field = User::display_name;
        let _: Field = User::description;
        let _: Field = User::metadata;
        let _: Field = User::created_at;
        let _: Field = User::updated_at;
        let _: Field = User::version;
    }

    #[test]
    fn field_consts_exist() {
        let _: Field = User::id;
        let _: Field = User::email;
        let _: Field = User::avatar;
        let _: Field = User::active;
        let _: Field = User::password_hash;
    }

    #[test]
    fn field_const_values() {
        assert_eq!(User::id.widget, "readonly");
        assert_eq!(User::email.widget, "email");
        assert_eq!(User::avatar.widget, "image");
        assert_eq!(User::active.widget, "switch");
        assert_eq!(User::password_hash.widget, "hidden");
        assert_eq!(User::display_name.widget, "text");
        assert_eq!(User::created_at.widget, "datetime");
        assert_eq!(User::version.widget, "readonly");
    }

    #[test]
    fn dsl_metadata() {
        assert_eq!(User::__DSL_MODULE, "auth");
        assert_eq!(User::__DSL_NAME, "User");
        assert_eq!(User::__DSL_RESOURCE, "user");
    }

    #[test]
    fn dsl_ir_includes_common_fields() {
        let ir = User::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"display_name"), "IR has display_name: {:?}", names);
        assert!(names.contains(&"description"), "IR has description: {:?}", names);
        assert!(names.contains(&"metadata"), "IR has metadata: {:?}", names);
        assert!(names.contains(&"created_at"), "IR has created_at: {:?}", names);
        assert!(names.contains(&"updated_at"), "IR has updated_at: {:?}", names);
        assert!(names.contains(&"version"), "IR has version: {:?}", names);
    }

    // ── Model with explicit #[ui(widget)] ──

    #[model(module = "auth")]
    pub struct Role {
        pub id: Id,
        #[ui(widget = "permission_picker")]
        pub permissions: Vec<String>,
    }

    #[test]
    fn explicit_ui_widget() {
        assert_eq!(Role::permissions.widget, "permission_picker");
    }
}
