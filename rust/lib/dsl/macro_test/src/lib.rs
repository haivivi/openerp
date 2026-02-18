//! Integration tests for the #[model] and #[facet] macros.

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
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"passwordHash\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"displayName\""));
        assert!(json.contains("\"alice@test.com\""));
    }

    #[test]
    fn common_fields_auto_injected() {
        let _: Field = User::display_name;
        let _: Field = User::description;
        let _: Field = User::metadata;
        let _: Field = User::created_at;
        let _: Field = User::updated_at;
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

// ── DslEnum macro tests ───────────────────────────────────────────

#[cfg(test)]
mod enum_tests {
    use openerp_macro::dsl_enum;
    use openerp_types::DslEnum;

    #[dsl_enum(module = "test")]
    pub enum Priority {
        Low,
        Medium,
        High,
        Critical,
    }

    #[test]
    fn enum_serde_screaming_snake() {
        let json = serde_json::to_string(&Priority::High).unwrap();
        assert_eq!(json, "\"HIGH\"");
        let back: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Priority::High);
    }

    #[test]
    fn enum_serde_in_progress_style() {
        #[dsl_enum(module = "test")]
        pub enum Status {
            Draft,
            InProgress,
            Completed,
        }
        let json = serde_json::to_string(&Status::InProgress).unwrap();
        assert_eq!(json, "\"IN_PROGRESS\"");
        let back: Status = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Status::InProgress);
    }

    #[test]
    fn enum_display_and_from_str() {
        assert_eq!(Priority::Low.to_string(), "LOW");
        assert_eq!(Priority::Critical.to_string(), "CRITICAL");

        let parsed: Priority = "high".parse().unwrap();
        assert_eq!(parsed, Priority::High);
        let parsed: Priority = "HIGH".parse().unwrap();
        assert_eq!(parsed, Priority::High);
        let parsed: Priority = "High".parse().unwrap();
        assert_eq!(parsed, Priority::High);
    }

    #[test]
    fn enum_from_str_error() {
        let err = "nonexistent".parse::<Priority>().unwrap_err();
        assert!(err.contains("unknown Priority variant"));
    }

    #[test]
    fn enum_default_is_first_variant() {
        assert_eq!(Priority::default(), Priority::Low);
    }

    #[test]
    fn enum_variants() {
        assert_eq!(Priority::variants(), &["LOW", "MEDIUM", "HIGH", "CRITICAL"]);
    }

    #[test]
    fn enum_dsl_trait() {
        assert_eq!(Priority::module(), "test");
        assert_eq!(Priority::enum_name(), "Priority");
        assert_eq!(<Priority as DslEnum>::variants(), &["LOW", "MEDIUM", "HIGH", "CRITICAL"]);
    }

    #[test]
    fn enum_dsl_ir() {
        let ir = Priority::__dsl_ir();
        assert_eq!(ir["type"], "enum");
        assert_eq!(ir["name"], "Priority");
        assert_eq!(ir["module"], "test");
        let variants = ir["variants"].as_array().unwrap();
        assert_eq!(variants.len(), 4);
        assert_eq!(variants[0], "LOW");
        assert_eq!(variants[3], "CRITICAL");
    }

    #[test]
    fn enum_in_model() {
        use openerp_macro::model;
        use openerp_types::*;

        #[dsl_enum(module = "test")]
        pub enum ItemStatus {
            Draft,
            Active,
            Archived,
        }

        #[model(module = "test")]
        pub struct TestItem {
            pub id: Id,
            pub status: ItemStatus,
        }

        assert_eq!(TestItem::status.widget, "select");

        let item = TestItem {
            id: Id::new("t1"),
            status: ItemStatus::Active,
            display_name: None,
            description: None,
            metadata: None,
            created_at: DateTime::default(),
            updated_at: DateTime::default(),
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["status"], "ACTIVE");

        let back: TestItem = serde_json::from_value(json).unwrap();
        assert_eq!(back.status, ItemStatus::Active);
    }
}

// ── Facet macro tests ──────────────────────────────────────────────

#[cfg(test)]
mod facet_tests {
    use openerp_macro::facet;

    // ── Action request/response types (defined before facet module) ──

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TestProvisionRequest {
        pub count: Option<u32>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TestProvisionResponse {
        pub batch_id: String,
        pub provisioned: u32,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct TestActivateResponse {
        pub sn: String,
        pub status: String,
    }

    // ── Facet definition ──

    #[facet(name = "mfg", module = "pms")]
    pub mod mfg {
        use super::*;

        /// Product model for MFG app.
        #[resource(path = "/models", pk = "code")]
        pub struct MfgModel {
            pub code: u32,
            pub series_name: String,
            pub display_name: Option<String>,
        }

        /// Production batch for MFG app.
        #[resource(path = "/batches", pk = "id", singular = "batch")]
        pub struct MfgBatch {
            pub id: String,
            pub model: u32,
            pub quantity: u32,
            pub status: String,
        }

        /// Device view for MFG app.
        #[resource(path = "/devices", pk = "sn")]
        pub struct MfgDevice {
            pub sn: String,
            pub model: u32,
            pub status: String,
        }

        #[action(method = "POST", path = "/batches/{id}/@provision")]
        pub type Provision = fn(id: String, req: TestProvisionRequest) -> TestProvisionResponse;

        #[action(method = "POST", path = "/devices/{sn}/@activate")]
        pub type Activate = fn(sn: String) -> TestActivateResponse;
    }

    // ── Tests ──

    #[test]
    fn facet_metadata() {
        assert_eq!(mfg::__FACET_NAME, "mfg");
        assert_eq!(mfg::__FACET_MODULE, "pms");
    }

    #[test]
    fn facet_ir_structure() {
        let ir = mfg::__facet_ir();
        assert_eq!(ir["name"], "mfg");
        assert_eq!(ir["module"], "pms");

        let resources = ir["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 3);
        assert_eq!(resources[0]["name"], "MfgModel");
        assert_eq!(resources[0]["path"], "/models");
        assert_eq!(resources[0]["pk"], "code");
        assert_eq!(resources[1]["name"], "MfgBatch");
        assert_eq!(resources[2]["name"], "MfgDevice");

        let actions = ir["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0]["name"], "provision");
        assert_eq!(actions[0]["method"], "POST");
        assert_eq!(actions[0]["path"], "/batches/{id}/@provision");
        assert_eq!(actions[0]["hasBody"], true);
        assert_eq!(actions[1]["name"], "activate");
        assert_eq!(actions[1]["hasBody"], false);
    }

    #[test]
    fn resource_struct_has_serde() {
        // MfgModel should have camelCase serde.
        let model = mfg::MfgModel {
            code: 42,
            series_name: "H106".into(),
            display_name: Some("Speaker".into()),
        };
        let json = serde_json::to_value(&model).unwrap();
        assert_eq!(json["code"], 42);
        assert_eq!(json["seriesName"], "H106");
        assert_eq!(json["displayName"], "Speaker");

        // Deserialize back.
        let back: mfg::MfgModel = serde_json::from_value(json).unwrap();
        assert_eq!(back.code, 42);
        assert_eq!(back.series_name, "H106");
    }

    #[test]
    fn resource_struct_camel_case_deserialize() {
        let json = r#"{"code":1001,"seriesName":"H200","displayName":null}"#;
        let model: mfg::MfgModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.code, 1001);
        assert_eq!(model.series_name, "H200");
        assert!(model.display_name.is_none());
    }

    #[test]
    fn client_struct_exists() {
        // Verify the MfgClient struct exists and can be constructed.
        // We can't actually call methods (no server), but we can check it compiles.
        fn _assert_client_compiles() {
            let _client = mfg::MfgClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::NoAuth),
            );
        }
    }

    #[test]
    fn batch_serde() {
        let batch = mfg::MfgBatch {
            id: "b001".into(),
            model: 42,
            quantity: 100,
            status: "pending".into(),
        };
        let json = serde_json::to_string(&batch).unwrap();
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"model\""));
        let back: mfg::MfgBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "b001");
    }
}
