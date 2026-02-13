//! Integration tests for the #[model] macro.

#[cfg(test)]
mod tests {
    use openerp_macro::model;
    use openerp_types::*;

    // ── Basic model ──

    #[model(module = "auth")]
    pub struct User {
        pub id: Id,
        pub name: String,
        pub email: Option<Email>,
        pub avatar: Option<Avatar>,
        pub active: bool,
        pub password_hash: Option<PasswordHash>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[test]
    fn model_has_serde() {
        let user = User {
            id: Id::new("u1"),
            name: "Alice".into(),
            email: Some(Email::new("alice@test.com")),
            avatar: None,
            active: true,
            password_hash: None,
            created_at: DateTime::new("2024-01-01T00:00:00Z"),
            updated_at: DateTime::new("2024-01-01T00:00:00Z"),
        };
        let json = serde_json::to_string(&user).unwrap();
        // camelCase
        assert!(json.contains("\"passwordHash\""));
        assert!(json.contains("\"createdAt\""));
        // Transparent newtypes serialize as plain strings
        assert!(json.contains("\"alice@test.com\""));
    }

    #[test]
    fn field_consts_exist() {
        // These are compile-time checked — if they don't exist, this won't compile.
        let _: Field = User::id;
        let _: Field = User::name;
        let _: Field = User::email;
        let _: Field = User::avatar;
        let _: Field = User::active;
        let _: Field = User::password_hash;
        let _: Field = User::created_at;
        let _: Field = User::updated_at;
    }

    #[test]
    fn field_const_values() {
        assert_eq!(User::id.name, "id");
        assert_eq!(User::id.widget, "readonly"); // Id -> readonly
        assert_eq!(User::email.name, "email");
        assert_eq!(User::email.widget, "email"); // Email -> email
        assert_eq!(User::avatar.widget, "image"); // Avatar -> image
        assert_eq!(User::active.widget, "switch"); // bool -> switch
        assert_eq!(User::password_hash.widget, "hidden"); // PasswordHash -> hidden
        assert_eq!(User::created_at.widget, "datetime"); // DateTime -> datetime
    }

    #[test]
    fn dsl_metadata() {
        assert_eq!(User::__DSL_MODULE, "auth");
        assert_eq!(User::__DSL_NAME, "User");
        assert_eq!(User::__DSL_RESOURCE, "user");
    }

    #[test]
    fn dsl_ir_json() {
        let ir = User::__dsl_ir();
        assert_eq!(ir["name"], "User");
        assert_eq!(ir["module"], "auth");
        assert_eq!(ir["resource"], "user");

        let fields = ir["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 8);
        assert_eq!(fields[0]["name"], "id");
        assert_eq!(fields[0]["widget"], "readonly");
        assert_eq!(fields[2]["name"], "email");
        assert_eq!(fields[2]["widget"], "email");
    }

    // ── Model with explicit #[ui(widget)] ──

    #[model(module = "auth")]
    pub struct Role {
        pub id: Id,
        pub description: Option<String>,
        #[ui(widget = "permission_picker")]
        pub permissions: Vec<String>,
    }

    #[test]
    fn explicit_ui_widget() {
        assert_eq!(Role::permissions.widget, "permission_picker");
        assert_eq!(Role::description.widget, "textarea"); // "description" field name heuristic
    }

    #[test]
    fn vec_string_default_widget() {
        // Without #[ui], Vec<String> would be "tags"
        // But Role.permissions has explicit override
        let ir = Role::__dsl_ir();
        let fields = ir["fields"].as_array().unwrap();
        let perm_field = fields.iter().find(|f| f["name"] == "permissions").unwrap();
        assert_eq!(perm_field["widget"], "permission_picker");
    }
}
