//! Facet DSL golden tests — validates #[facet], #[resource], #[action] macro.
//!
//! Tests the full facet framework lifecycle:
//!   1. #[facet] generates serde-derived resource structs
//!   2. #[resource] metadata: path, pk, camelCase serde
//!   3. #[action] metadata: method, path, params, body detection
//!   4. __facet_ir() returns correct JSON structure
//!   5. Client struct exists and compiles with correct method signatures
//!   6. Multiple facets coexist independently
//!   7. Edge cases: no resources, no actions, resources only, actions only
//!   8. Facet + admin model integration (facet projects from #[model] types)

#[cfg(test)]
mod tests {
    use openerp_macro::facet;

    // =====================================================================
    // Action request/response types (shared across facet definitions)
    // =====================================================================

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ProvisionRequest {
        pub count: Option<u32>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ProvisionResponse {
        pub batch_id: String,
        pub provisioned: u32,
        pub devices: Vec<String>,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ActivateResponse {
        pub sn: String,
        pub status: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct PublishRequest {
        pub visibility: String,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct PublishResponse {
        pub id: String,
        pub published: bool,
    }

    // =====================================================================
    // Facet: MFG — full-featured (4 resources + 2 actions)
    // =====================================================================

    #[facet(name = "mfg", module = "pms")]
    pub mod mfg {
        use super::*;

        /// Product model for the factory floor.
        #[resource(path = "/models", pk = "code")]
        pub struct MfgModel {
            pub code: u32,
            pub series_name: String,
            pub display_name: Option<String>,
        }

        /// Production batch tracking.
        #[resource(path = "/batches", pk = "id", singular = "batch")]
        pub struct MfgBatch {
            pub id: String,
            pub model: u32,
            pub quantity: u32,
            pub provisioned_count: u32,
            pub status: String,
        }

        /// Device view — no secrets exposed.
        #[resource(path = "/devices", pk = "sn")]
        pub struct MfgDevice {
            pub sn: String,
            pub model: u32,
            pub status: String,
            pub sku: Option<String>,
            pub imei: Vec<String>,
        }

        /// Firmware versions for flashing.
        #[resource(path = "/firmwares", pk = "id")]
        pub struct MfgFirmware {
            pub id: String,
            pub model: u32,
            pub semver: String,
            pub build: u64,
            pub status: String,
        }

        #[action(method = "POST", path = "/batches/{id}/@provision")]
        pub type Provision = fn(id: String, req: ProvisionRequest) -> ProvisionResponse;

        #[action(method = "POST", path = "/devices/{sn}/@activate")]
        pub type Activate = fn(sn: String) -> ActivateResponse;
    }

    // =====================================================================
    // Facet: App — different consumer, different projection
    // =====================================================================

    #[facet(name = "app", module = "km")]
    pub mod app_km {
        use super::*;

        /// Public document listing for the mobile app.
        #[resource(path = "/articles", pk = "id")]
        pub struct AppArticle {
            pub id: String,
            pub title: String,
            pub summary: Option<String>,
            pub published: bool,
            pub tags: Vec<String>,
        }

        #[action(method = "POST", path = "/articles/{id}/@publish")]
        pub type Publish = fn(id: String, req: PublishRequest) -> PublishResponse;
    }

    // =====================================================================
    // Facet: Minimal — resources only, no actions
    // =====================================================================

    #[facet(name = "public", module = "org")]
    pub mod public_org {
        /// Public company info.
        #[resource(path = "/companies", pk = "id", singular = "company")]
        pub struct PublicCompany {
            pub id: String,
            pub name: String,
            pub website: Option<String>,
        }
    }

    // =====================================================================
    // Facet: Actions only — no resources
    // =====================================================================

    #[facet(name = "webhook", module = "notify")]
    pub mod webhook_notify {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct PingResponse {
            pub ok: bool,
        }

        #[action(method = "POST", path = "/@ping")]
        pub type Ping = fn() -> PingResponse;
    }

    // =====================================================================
    // Golden: Facet metadata
    // =====================================================================

    #[test]
    fn golden_facet_metadata_mfg() {
        assert_eq!(mfg::__FACET_NAME, "mfg");
        assert_eq!(mfg::__FACET_MODULE, "pms");
    }

    #[test]
    fn golden_facet_metadata_app() {
        assert_eq!(app_km::__FACET_NAME, "app");
        assert_eq!(app_km::__FACET_MODULE, "km");
    }

    #[test]
    fn golden_facet_metadata_public() {
        assert_eq!(public_org::__FACET_NAME, "public");
        assert_eq!(public_org::__FACET_MODULE, "org");
    }

    #[test]
    fn golden_facet_metadata_webhook() {
        assert_eq!(webhook_notify::__FACET_NAME, "webhook");
        assert_eq!(webhook_notify::__FACET_MODULE, "notify");
    }

    // =====================================================================
    // Golden: Facet IR structure
    // =====================================================================

    #[test]
    fn golden_mfg_ir_resources() {
        let ir = mfg::__facet_ir();
        assert_eq!(ir["name"], "mfg");
        assert_eq!(ir["module"], "pms");

        let resources = ir["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 4);

        // Resource order matches definition order.
        assert_eq!(resources[0]["name"], "MfgModel");
        assert_eq!(resources[0]["path"], "/models");
        assert_eq!(resources[0]["pk"], "code");

        assert_eq!(resources[1]["name"], "MfgBatch");
        assert_eq!(resources[1]["path"], "/batches");
        assert_eq!(resources[1]["pk"], "id");

        assert_eq!(resources[2]["name"], "MfgDevice");
        assert_eq!(resources[2]["path"], "/devices");
        assert_eq!(resources[2]["pk"], "sn");

        assert_eq!(resources[3]["name"], "MfgFirmware");
        assert_eq!(resources[3]["path"], "/firmwares");
        assert_eq!(resources[3]["pk"], "id");
    }

    #[test]
    fn golden_mfg_ir_actions() {
        let ir = mfg::__facet_ir();
        let actions = ir["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 2);

        assert_eq!(actions[0]["name"], "provision");
        assert_eq!(actions[0]["method"], "POST");
        assert_eq!(actions[0]["path"], "/batches/{id}/@provision");
        assert_eq!(actions[0]["hasBody"], true);

        assert_eq!(actions[1]["name"], "activate");
        assert_eq!(actions[1]["method"], "POST");
        assert_eq!(actions[1]["path"], "/devices/{sn}/@activate");
        assert_eq!(actions[1]["hasBody"], false);
    }

    #[test]
    fn golden_app_ir() {
        let ir = app_km::__facet_ir();
        assert_eq!(ir["name"], "app");
        assert_eq!(ir["module"], "km");

        let resources = ir["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0]["name"], "AppArticle");
        assert_eq!(resources[0]["path"], "/articles");

        let actions = ir["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["name"], "publish");
        assert_eq!(actions[0]["hasBody"], true);
    }

    #[test]
    fn golden_resources_only_ir() {
        let ir = public_org::__facet_ir();
        let resources = ir["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 1);
        let actions = ir["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 0, "No actions defined");
    }

    #[test]
    fn golden_actions_only_ir() {
        let ir = webhook_notify::__facet_ir();
        let resources = ir["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 0, "No resources defined");
        let actions = ir["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["name"], "ping");
        assert_eq!(actions[0]["hasBody"], false);
    }

    // =====================================================================
    // Golden: Resource struct serde (camelCase + roundtrip)
    // =====================================================================

    #[test]
    fn golden_resource_serde_camel_case() {
        let model = mfg::MfgModel {
            code: 1001,
            series_name: "H106".into(),
            display_name: Some("H106 Speaker".into()),
        };
        let json = serde_json::to_value(&model).unwrap();
        assert_eq!(json["code"], 1001);
        assert_eq!(json["seriesName"], "H106");
        assert_eq!(json["displayName"], "H106 Speaker");
        // No snake_case keys.
        assert!(json.get("series_name").is_none());
        assert!(json.get("display_name").is_none());
    }

    #[test]
    fn golden_resource_serde_deserialize() {
        let json = r#"{"code":42,"seriesName":"X200","displayName":null}"#;
        let model: mfg::MfgModel = serde_json::from_str(json).unwrap();
        assert_eq!(model.code, 42);
        assert_eq!(model.series_name, "X200");
        assert!(model.display_name.is_none());
    }

    #[test]
    fn golden_resource_serde_roundtrip() {
        let device = mfg::MfgDevice {
            sn: "SN001".into(),
            model: 42,
            status: "provisioned".into(),
            sku: Some("SKU-A".into()),
            imei: vec!["860000001".into(), "860000002".into()],
        };
        let json = serde_json::to_string(&device).unwrap();
        let back: mfg::MfgDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sn, "SN001");
        assert_eq!(back.model, 42);
        assert_eq!(back.imei.len(), 2);
        assert_eq!(back.sku, Some("SKU-A".into()));
    }

    #[test]
    fn golden_resource_all_types() {
        // MfgFirmware has u64 + String + Option fields.
        let fw = mfg::MfgFirmware {
            id: "fw001".into(),
            model: 42,
            semver: "1.2.3".into(),
            build: 9999,
            status: "uploaded".into(),
        };
        let json = serde_json::to_value(&fw).unwrap();
        assert_eq!(json["build"], 9999);
        assert_eq!(json["semver"], "1.2.3");

        let back: mfg::MfgFirmware = serde_json::from_value(json).unwrap();
        assert_eq!(back.build, 9999);
    }

    #[test]
    fn golden_resource_vec_field() {
        let article = app_km::AppArticle {
            id: "a1".into(),
            title: "Test".into(),
            summary: None,
            published: true,
            tags: vec!["rust".into(), "dsl".into()],
        };
        let json = serde_json::to_value(&article).unwrap();
        assert_eq!(json["tags"].as_array().unwrap().len(), 2);
        assert_eq!(json["published"], true);
    }

    // =====================================================================
    // Golden: Client struct compilation
    // =====================================================================

    #[test]
    fn golden_client_compiles_mfg() {
        // MfgClient should exist with new().
        fn _check() {
            let _c = mfg::MfgClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::NoAuth),
            );
        }
    }

    #[test]
    fn golden_client_compiles_app() {
        fn _check() {
            let _c = app_km::AppClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::StaticToken::new("tok")),
            );
        }
    }

    #[test]
    fn golden_client_compiles_public() {
        fn _check() {
            let _c = public_org::PublicClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::NoAuth),
            );
        }
    }

    #[test]
    fn golden_client_compiles_webhook() {
        fn _check() {
            let _c = webhook_notify::WebhookClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::NoAuth),
            );
        }
    }

    // =====================================================================
    // Golden: Client method signatures (compile-time verification)
    // =====================================================================

    #[test]
    fn golden_client_method_signatures() {
        // Verify that the generated methods have the expected signatures.
        // This is a compile-time check — if signatures are wrong, it won't compile.
        async fn _verify_mfg(c: &mfg::MfgClient) {
            let _: Result<openerp_client::ListResult<mfg::MfgModel>, _> = c.list_models().await;
            let _: Result<mfg::MfgModel, _> = c.get_model("1001").await;
            let _: Result<openerp_client::ListResult<mfg::MfgBatch>, _> = c.list_batches().await;
            let _: Result<mfg::MfgBatch, _> = c.get_batch("b1").await;
            let _: Result<openerp_client::ListResult<mfg::MfgDevice>, _> = c.list_devices().await;
            let _: Result<mfg::MfgDevice, _> = c.get_device("SN1").await;
            let _: Result<openerp_client::ListResult<mfg::MfgFirmware>, _> = c.list_firmwares().await;
            let _: Result<mfg::MfgFirmware, _> = c.get_firmware("fw1").await;

            let req = ProvisionRequest { count: Some(10) };
            let _: Result<ProvisionResponse, _> = c.provision("b1", &req).await;
            let _: Result<ActivateResponse, _> = c.activate("SN1").await;
        }

        async fn _verify_app(c: &app_km::AppClient) {
            let _: Result<openerp_client::ListResult<app_km::AppArticle>, _> = c.list_articles().await;
            let _: Result<app_km::AppArticle, _> = c.get_article("a1").await;

            let req = PublishRequest { visibility: "public".into() };
            let _: Result<PublishResponse, _> = c.publish("a1", &req).await;
        }

        async fn _verify_public(c: &public_org::PublicClient) {
            let _: Result<openerp_client::ListResult<public_org::PublicCompany>, _> = c.list_companies().await;
            let _: Result<public_org::PublicCompany, _> = c.get_company("c1").await;
        }

        async fn _verify_webhook(c: &webhook_notify::WebhookClient) {
            let _: Result<webhook_notify::PingResponse, _> = c.ping().await;
        }
    }

    // =====================================================================
    // Golden: Multiple facets coexist — no name collisions
    // =====================================================================

    #[test]
    fn golden_facet_isolation() {
        // Each facet has its own module, its own IR, its own client.
        let mfg_ir = mfg::__facet_ir();
        let app_ir = app_km::__facet_ir();
        let public_ir = public_org::__facet_ir();
        let webhook_ir = webhook_notify::__facet_ir();

        assert_ne!(mfg_ir["name"], app_ir["name"]);
        assert_ne!(mfg_ir["module"], app_ir["module"]);
        assert_ne!(public_ir["name"], webhook_ir["name"]);

        // Resources don't leak across facets.
        assert_eq!(mfg_ir["resources"].as_array().unwrap().len(), 4);
        assert_eq!(app_ir["resources"].as_array().unwrap().len(), 1);
        assert_eq!(public_ir["resources"].as_array().unwrap().len(), 1);
        assert_eq!(webhook_ir["resources"].as_array().unwrap().len(), 0);
    }

    // =====================================================================
    // Golden: Non-resource items pass through unchanged
    // =====================================================================

    #[facet(name = "mixed", module = "test")]
    pub mod mixed {
        /// A plain struct — not annotated with #[resource], should pass through.
        #[derive(Debug, Clone)]
        pub struct HelperType {
            pub value: i32,
        }

        /// A resource.
        #[resource(path = "/items", pk = "id")]
        pub struct MixedItem {
            pub id: String,
            pub name: String,
        }

        /// A constant.
        pub const MAX_ITEMS: usize = 100;

        /// A function.
        pub fn format_id(id: &str) -> String {
            format!("mixed-{}", id)
        }
    }

    #[test]
    fn golden_pass_through_items() {
        // Non-resource items are available.
        let h = mixed::HelperType { value: 42 };
        assert_eq!(h.value, 42);

        assert_eq!(mixed::MAX_ITEMS, 100);
        assert_eq!(mixed::format_id("abc"), "mixed-abc");

        // Resource item also works.
        let item = mixed::MixedItem {
            id: "i1".into(),
            name: "Test".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"id\""));

        // IR only has the resource, not the helper.
        let ir = mixed::__facet_ir();
        assert_eq!(ir["resources"].as_array().unwrap().len(), 1);
        assert_eq!(ir["actions"].as_array().unwrap().len(), 0);
    }
}
