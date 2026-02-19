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
            let _: Result<openerp_client::ListResult<mfg::MfgModel>, _> = c.list_models(None).await;
            let _: Result<mfg::MfgModel, _> = c.get_model("1001").await;
            let _: Result<openerp_client::ListResult<mfg::MfgBatch>, _> = c.list_batches(None).await;
            let _: Result<mfg::MfgBatch, _> = c.get_batch("b1").await;
            let _: Result<openerp_client::ListResult<mfg::MfgDevice>, _> = c.list_devices(None).await;
            let _: Result<mfg::MfgDevice, _> = c.get_device("SN1").await;
            let _: Result<openerp_client::ListResult<mfg::MfgFirmware>, _> = c.list_firmwares(None).await;
            let _: Result<mfg::MfgFirmware, _> = c.get_firmware("fw1").await;

            let req = ProvisionRequest { count: Some(10) };
            let _: Result<ProvisionResponse, _> = c.provision("b1", &req).await;
            let _: Result<ActivateResponse, _> = c.activate("SN1").await;
        }

        async fn _verify_app(c: &app_km::AppClient) {
            let _: Result<openerp_client::ListResult<app_km::AppArticle>, _> = c.list_articles(None).await;
            let _: Result<app_km::AppArticle, _> = c.get_article("a1").await;

            let req = PublishRequest { visibility: "public".into() };
            let _: Result<PublishResponse, _> = c.publish("a1", &req).await;
        }

        async fn _verify_public(c: &public_org::PublicClient) {
            let _: Result<openerp_client::ListResult<public_org::PublicCompany>, _> = c.list_companies(None).await;
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

    // =====================================================================
    // Golden: FlatBuffer encode/decode roundtrip
    // =====================================================================

    #[test]
    fn golden_flatbuffer_roundtrip_simple() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let model = mfg::MfgModel {
            code: 1001,
            series_name: "H106".into(),
            display_name: Some("H106 Speaker".into()),
        };

        let buf = model.encode_flatbuffer();
        assert!(!buf.is_empty());

        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.code, 1001);
        assert_eq!(decoded.series_name, "H106");
        assert_eq!(decoded.display_name, Some("H106 Speaker".into()));
    }

    #[test]
    fn golden_flatbuffer_roundtrip_all_field_types() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let device = mfg::MfgDevice {
            sn: "SN-ABCD-1234".into(),
            model: 42,
            status: "provisioned".into(),
            sku: Some("SKU-A".into()),
            imei: vec!["860000001".into(), "860000002".into()],
        };

        let buf = device.encode_flatbuffer();
        let decoded = mfg::MfgDevice::decode_flatbuffer(&buf).unwrap();

        assert_eq!(decoded.sn, "SN-ABCD-1234");
        assert_eq!(decoded.model, 42);
        assert_eq!(decoded.status, "provisioned");
        assert_eq!(decoded.sku, Some("SKU-A".into()));
        assert_eq!(decoded.imei, vec!["860000001", "860000002"]);
    }

    #[test]
    fn golden_flatbuffer_roundtrip_option_none() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let model = mfg::MfgModel {
            code: 0,
            series_name: "X".into(),
            display_name: None,
        };

        let buf = model.encode_flatbuffer();
        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.code, 0);
        assert_eq!(decoded.series_name, "X");
        assert_eq!(decoded.display_name, None);
    }

    #[test]
    fn golden_flatbuffer_roundtrip_empty_vec() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let device = mfg::MfgDevice {
            sn: "SN1".into(),
            model: 1,
            status: "new".into(),
            sku: None,
            imei: vec![],
        };

        let buf = device.encode_flatbuffer();
        let decoded = mfg::MfgDevice::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.sn, "SN1");
        assert!(decoded.sku.is_none());
        assert!(decoded.imei.is_empty());
    }

    #[test]
    fn golden_flatbuffer_roundtrip_u64_field() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let fw = mfg::MfgFirmware {
            id: "fw-001".into(),
            model: 7,
            semver: "2.0.0-beta".into(),
            build: 999_999_999,
            status: "uploaded".into(),
        };

        let buf = fw.encode_flatbuffer();
        let decoded = mfg::MfgFirmware::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.id, "fw-001");
        assert_eq!(decoded.model, 7);
        assert_eq!(decoded.semver, "2.0.0-beta");
        assert_eq!(decoded.build, 999_999_999);
        assert_eq!(decoded.status, "uploaded");
    }

    // =====================================================================
    // Golden: FlatBuffer ↔ JSON equivalence
    // =====================================================================

    #[test]
    fn golden_flatbuffer_json_equivalence() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let batch = mfg::MfgBatch {
            id: "b-100".into(),
            model: 42,
            quantity: 500,
            provisioned_count: 123,
            status: "in_progress".into(),
        };

        // JSON roundtrip.
        let json_str = serde_json::to_string(&batch).unwrap();
        let json_back: mfg::MfgBatch = serde_json::from_str(&json_str).unwrap();

        // FlatBuffer roundtrip.
        let fb_buf = batch.encode_flatbuffer();
        let fb_back = mfg::MfgBatch::decode_flatbuffer(&fb_buf).unwrap();

        // Both should produce identical data.
        assert_eq!(json_back.id, fb_back.id);
        assert_eq!(json_back.model, fb_back.model);
        assert_eq!(json_back.quantity, fb_back.quantity);
        assert_eq!(json_back.provisioned_count, fb_back.provisioned_count);
        assert_eq!(json_back.status, fb_back.status);
    }

    #[test]
    fn golden_flatbuffer_json_equivalence_with_optionals() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let device = mfg::MfgDevice {
            sn: "SN-TEST".into(),
            model: 99,
            status: "active".into(),
            sku: Some("PREMIUM".into()),
            imei: vec!["111".into(), "222".into(), "333".into()],
        };

        let json_str = serde_json::to_string(&device).unwrap();
        let json_back: mfg::MfgDevice = serde_json::from_str(&json_str).unwrap();

        let fb_buf = device.encode_flatbuffer();
        let fb_back = mfg::MfgDevice::decode_flatbuffer(&fb_buf).unwrap();

        assert_eq!(json_back.sn, fb_back.sn);
        assert_eq!(json_back.model, fb_back.model);
        assert_eq!(json_back.status, fb_back.status);
        assert_eq!(json_back.sku, fb_back.sku);
        assert_eq!(json_back.imei, fb_back.imei);
    }

    // =====================================================================
    // Golden: FlatBuffer list encode/decode
    // =====================================================================

    #[test]
    fn golden_flatbuffer_list_roundtrip() {
        use openerp_types::{IntoFlatBufferList, FromFlatBufferList};

        let items = vec![
            mfg::MfgModel {
                code: 1,
                series_name: "A".into(),
                display_name: Some("Model A".into()),
            },
            mfg::MfgModel {
                code: 2,
                series_name: "B".into(),
                display_name: None,
            },
        ];

        let buf = mfg::MfgModel::encode_flatbuffer_list(&items, true);
        let (decoded, has_more) = mfg::MfgModel::decode_flatbuffer_list(&buf).unwrap();

        assert!(has_more);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].code, 1);
        assert_eq!(decoded[0].series_name, "A");
        assert_eq!(decoded[0].display_name, Some("Model A".into()));
        assert_eq!(decoded[1].code, 2);
        assert_eq!(decoded[1].series_name, "B");
        assert_eq!(decoded[1].display_name, None);
    }

    #[test]
    fn golden_flatbuffer_list_empty() {
        use openerp_types::{IntoFlatBufferList, FromFlatBufferList};

        let items: Vec<mfg::MfgModel> = vec![];
        let buf = mfg::MfgModel::encode_flatbuffer_list(&items, false);
        let (decoded, has_more) = mfg::MfgModel::decode_flatbuffer_list(&buf).unwrap();

        assert!(!has_more);
        assert!(decoded.is_empty());
    }

    #[test]
    fn golden_flatbuffer_list_complex_items() {
        use openerp_types::{IntoFlatBufferList, FromFlatBufferList};

        let items = vec![
            mfg::MfgDevice {
                sn: "SN1".into(),
                model: 1,
                status: "new".into(),
                sku: None,
                imei: vec![],
            },
            mfg::MfgDevice {
                sn: "SN2".into(),
                model: 2,
                status: "active".into(),
                sku: Some("PRO".into()),
                imei: vec!["111".into(), "222".into()],
            },
        ];

        let buf = mfg::MfgDevice::encode_flatbuffer_list(&items, false);
        let (decoded, has_more) = mfg::MfgDevice::decode_flatbuffer_list(&buf).unwrap();

        assert!(!has_more);
        assert_eq!(decoded.len(), 2);

        assert_eq!(decoded[0].sn, "SN1");
        assert!(decoded[0].sku.is_none());
        assert!(decoded[0].imei.is_empty());

        assert_eq!(decoded[1].sn, "SN2");
        assert_eq!(decoded[1].sku, Some("PRO".into()));
        assert_eq!(decoded[1].imei, vec!["111", "222"]);
    }

    // =====================================================================
    // Golden: FBS schema generation
    // =====================================================================

    #[test]
    fn golden_fbs_schema_exists() {
        // Schema constants are generated for each resource.
        assert!(mfg::__FBS_SCHEMA_MFGMODEL.contains("table MfgModel"));
        assert!(mfg::__FBS_SCHEMA_MFGMODEL.contains("series_name: string"));
        assert!(mfg::__FBS_SCHEMA_MFGMODEL.contains("code: uint"));
        assert!(mfg::__FBS_SCHEMA_MFGMODEL.contains("display_name: string"));
        assert!(mfg::__FBS_SCHEMA_MFGMODEL.contains("table MfgModelList"));

        assert!(mfg::__FBS_SCHEMA_MFGDEVICE.contains("table MfgDevice"));
        assert!(mfg::__FBS_SCHEMA_MFGDEVICE.contains("imei: [string]"));
    }

    // =====================================================================
    // Golden: Client .format() compiles
    // =====================================================================

    #[test]
    fn golden_client_format_builder() {
        fn _check() {
            let _c = mfg::MfgClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::NoAuth),
            )
            .format(openerp_types::Format::FlatBuffers);
        }
    }

    // =====================================================================
    // Golden: Multiple facets have independent FlatBuffer impls
    // =====================================================================

    #[test]
    fn golden_flatbuffer_cross_facet_isolation() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let article = app_km::AppArticle {
            id: "a1".into(),
            title: "Hello".into(),
            summary: Some("World".into()),
            published: true,
            tags: vec!["rust".into()],
        };

        let buf = article.encode_flatbuffer();
        let decoded = app_km::AppArticle::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.id, "a1");
        assert_eq!(decoded.title, "Hello");
        assert_eq!(decoded.summary, Some("World".into()));
        assert_eq!(decoded.published, true);
        assert_eq!(decoded.tags, vec!["rust"]);
    }

    // =====================================================================
    // Golden: Boundary — scalar edge values
    // =====================================================================

    #[test]
    fn golden_flatbuffer_boundary_zero_scalars() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        // All scalars at zero — tests FlatBuffer default-value handling.
        // FlatBuffer omits fields equal to default (0) in vtable optimization;
        // the reader must still return 0, not garbage.
        let batch = mfg::MfgBatch {
            id: "".into(),
            model: 0,
            quantity: 0,
            provisioned_count: 0,
            status: "".into(),
        };
        let buf = batch.encode_flatbuffer();
        let decoded = mfg::MfgBatch::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.model, 0);
        assert_eq!(decoded.quantity, 0);
        assert_eq!(decoded.provisioned_count, 0);
        assert_eq!(decoded.id, "");
        assert_eq!(decoded.status, "");
    }

    #[test]
    fn golden_flatbuffer_boundary_u32_max() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let model = mfg::MfgModel {
            code: u32::MAX,
            series_name: "max".into(),
            display_name: None,
        };
        let buf = model.encode_flatbuffer();
        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.code, u32::MAX);
    }

    #[test]
    fn golden_flatbuffer_boundary_u64_max() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let fw = mfg::MfgFirmware {
            id: "fw-max".into(),
            model: u32::MAX,
            semver: "255.255.255".into(),
            build: u64::MAX,
            status: "max".into(),
        };
        let buf = fw.encode_flatbuffer();
        let decoded = mfg::MfgFirmware::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.build, u64::MAX);
        assert_eq!(decoded.model, u32::MAX);
    }

    #[test]
    fn golden_flatbuffer_boundary_bool_field() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        // published = false — tests that bool false survives roundtrip
        // (FlatBuffer default for bool is false, so it may be omitted).
        let article_false = app_km::AppArticle {
            id: "a-false".into(),
            title: "Draft".into(),
            summary: None,
            published: false,
            tags: vec![],
        };
        let buf = article_false.encode_flatbuffer();
        let decoded = app_km::AppArticle::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.published, false);

        // published = true
        let article_true = app_km::AppArticle {
            id: "a-true".into(),
            title: "Live".into(),
            summary: None,
            published: true,
            tags: vec![],
        };
        let buf = article_true.encode_flatbuffer();
        let decoded = app_km::AppArticle::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.published, true);
    }

    // =====================================================================
    // Golden: Boundary — string edge cases
    // =====================================================================

    #[test]
    fn golden_flatbuffer_boundary_empty_string() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let model = mfg::MfgModel {
            code: 1,
            series_name: "".into(),
            display_name: Some("".into()),
        };
        let buf = model.encode_flatbuffer();
        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.series_name, "");
        assert_eq!(decoded.display_name, Some("".into()));
    }

    #[test]
    fn golden_flatbuffer_boundary_unicode_cjk() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let model = mfg::MfgModel {
            code: 888,
            series_name: "智能音箱 H106".into(),
            display_name: Some("海维维 · 智能设备".into()),
        };
        let buf = model.encode_flatbuffer();
        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.series_name, "智能音箱 H106");
        assert_eq!(decoded.display_name, Some("海维维 · 智能设备".into()));
    }

    #[test]
    fn golden_flatbuffer_boundary_unicode_emoji() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let device = mfg::MfgDevice {
            sn: "SN-🔥-001".into(),
            model: 1,
            status: "🚀 launched".into(),
            sku: Some("💎 premium".into()),
            imei: vec!["📱1".into(), "📱2".into()],
        };
        let buf = device.encode_flatbuffer();
        let decoded = mfg::MfgDevice::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.sn, "SN-🔥-001");
        assert_eq!(decoded.status, "🚀 launched");
        assert_eq!(decoded.sku, Some("💎 premium".into()));
        assert_eq!(decoded.imei, vec!["📱1", "📱2"]);
    }

    #[test]
    fn golden_flatbuffer_boundary_long_string() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let long = "x".repeat(10_000);
        let model = mfg::MfgModel {
            code: 1,
            series_name: long.clone(),
            display_name: Some(long.clone()),
        };
        let buf = model.encode_flatbuffer();
        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.series_name.len(), 10_000);
        assert_eq!(decoded.display_name.as_ref().unwrap().len(), 10_000);
    }

    #[test]
    fn golden_flatbuffer_boundary_string_with_null_bytes() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let with_null = "hello\0world".to_string();
        let model = mfg::MfgModel {
            code: 1,
            series_name: with_null.clone(),
            display_name: None,
        };
        let buf = model.encode_flatbuffer();
        let decoded = mfg::MfgModel::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.series_name, with_null);
    }

    // =====================================================================
    // Golden: Boundary — Vec edge cases
    // =====================================================================

    #[test]
    fn golden_flatbuffer_boundary_large_vec() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let imei: Vec<String> = (0..100).map(|i| format!("IMEI-{:04}", i)).collect();
        let device = mfg::MfgDevice {
            sn: "SN-BIG".into(),
            model: 1,
            status: "ok".into(),
            sku: None,
            imei,
        };
        let buf = device.encode_flatbuffer();
        let decoded = mfg::MfgDevice::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.imei.len(), 100);
        assert_eq!(decoded.imei[0], "IMEI-0000");
        assert_eq!(decoded.imei[99], "IMEI-0099");
    }

    #[test]
    fn golden_flatbuffer_boundary_vec_with_empty_strings() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let device = mfg::MfgDevice {
            sn: "SN1".into(),
            model: 1,
            status: "ok".into(),
            sku: None,
            imei: vec!["".into(), "a".into(), "".into()],
        };
        let buf = device.encode_flatbuffer();
        let decoded = mfg::MfgDevice::decode_flatbuffer(&buf).unwrap();
        assert_eq!(decoded.imei, vec!["", "a", ""]);
    }

    // =====================================================================
    // Golden: Boundary — list edge cases
    // =====================================================================

    #[test]
    fn golden_flatbuffer_list_single_item() {
        use openerp_types::{IntoFlatBufferList, FromFlatBufferList};

        let items = vec![mfg::MfgModel {
            code: 42,
            series_name: "solo".into(),
            display_name: None,
        }];
        let buf = mfg::MfgModel::encode_flatbuffer_list(&items, false);
        let (decoded, has_more) = mfg::MfgModel::decode_flatbuffer_list(&buf).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].code, 42);
        assert!(!has_more);
    }

    #[test]
    fn golden_flatbuffer_list_many_items() {
        use openerp_types::{IntoFlatBufferList, FromFlatBufferList};

        let items: Vec<mfg::MfgBatch> = (0..50)
            .map(|i| mfg::MfgBatch {
                id: format!("batch-{}", i),
                model: i,
                quantity: i * 10,
                provisioned_count: i * 5,
                status: if i % 2 == 0 { "even".into() } else { "odd".into() },
            })
            .collect();

        let buf = mfg::MfgBatch::encode_flatbuffer_list(&items, true);
        let (decoded, has_more) = mfg::MfgBatch::decode_flatbuffer_list(&buf).unwrap();

        assert!(has_more);
        assert_eq!(decoded.len(), 50);
        assert_eq!(decoded[0].id, "batch-0");
        assert_eq!(decoded[0].quantity, 0);
        assert_eq!(decoded[49].id, "batch-49");
        assert_eq!(decoded[49].quantity, 490);
        assert_eq!(decoded[49].status, "odd");
    }

    #[test]
    fn golden_flatbuffer_list_has_more_false_vs_true() {
        use openerp_types::{IntoFlatBufferList, FromFlatBufferList};

        let items = vec![mfg::MfgModel {
            code: 1,
            series_name: "x".into(),
            display_name: None,
        }];

        let buf_false = mfg::MfgModel::encode_flatbuffer_list(&items, false);
        let (_, hm_false) = mfg::MfgModel::decode_flatbuffer_list(&buf_false).unwrap();
        assert!(!hm_false);

        let buf_true = mfg::MfgModel::encode_flatbuffer_list(&items, true);
        let (_, hm_true) = mfg::MfgModel::decode_flatbuffer_list(&buf_true).unwrap();
        assert!(hm_true);
    }

    // =====================================================================
    // Golden: Error — decode garbage / truncated data
    // =====================================================================

    #[test]
    fn golden_flatbuffer_error_empty_buffer() {
        use openerp_types::FromFlatBuffer;

        let result = mfg::MfgModel::decode_flatbuffer(&[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("too small"), "got: {}", err.message);
    }

    #[test]
    fn golden_flatbuffer_error_too_short() {
        use openerp_types::FromFlatBuffer;

        let result = mfg::MfgModel::decode_flatbuffer(&[0x01, 0x02, 0x03]);
        assert!(result.is_err());
    }

    #[test]
    fn golden_flatbuffer_list_error_empty_buffer() {
        use openerp_types::FromFlatBufferList;

        let result = mfg::MfgModel::decode_flatbuffer_list(&[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("too small"), "got: {}", err.message);
    }

    #[test]
    fn golden_flatbuffer_list_error_too_short() {
        use openerp_types::FromFlatBufferList;

        let result = mfg::MfgModel::decode_flatbuffer_list(&[0xFF, 0xFF]);
        assert!(result.is_err());
    }

    // =====================================================================
    // Golden: JSON ↔ FlatBuffer equivalence — exhaustive field coverage
    // =====================================================================

    #[test]
    fn golden_flatbuffer_json_equivalence_all_none_empty() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        // Minimal data — everything at zero / empty / None.
        let device = mfg::MfgDevice {
            sn: "".into(),
            model: 0,
            status: "".into(),
            sku: None,
            imei: vec![],
        };

        let json_str = serde_json::to_string(&device).unwrap();
        let json_back: mfg::MfgDevice = serde_json::from_str(&json_str).unwrap();

        let fb_buf = device.encode_flatbuffer();
        let fb_back = mfg::MfgDevice::decode_flatbuffer(&fb_buf).unwrap();

        assert_eq!(json_back.sn, fb_back.sn);
        assert_eq!(json_back.model, fb_back.model);
        assert_eq!(json_back.status, fb_back.status);
        assert_eq!(json_back.sku, fb_back.sku);
        assert_eq!(json_back.imei, fb_back.imei);
    }

    #[test]
    fn golden_flatbuffer_json_equivalence_article_with_bool_vec() {
        use openerp_types::{IntoFlatBuffer, FromFlatBuffer};

        let article = app_km::AppArticle {
            id: "article-99".into(),
            title: "测试 Title".into(),
            summary: Some("概要 Summary".into()),
            published: false,
            tags: vec!["tag-a".into(), "tag-b".into(), "标签".into()],
        };

        let json_str = serde_json::to_string(&article).unwrap();
        let json_back: app_km::AppArticle = serde_json::from_str(&json_str).unwrap();

        let fb_buf = article.encode_flatbuffer();
        let fb_back = app_km::AppArticle::decode_flatbuffer(&fb_buf).unwrap();

        assert_eq!(json_back.id, fb_back.id);
        assert_eq!(json_back.title, fb_back.title);
        assert_eq!(json_back.summary, fb_back.summary);
        assert_eq!(json_back.published, fb_back.published);
        assert_eq!(json_back.tags, fb_back.tags);
    }

    // =====================================================================
    // Golden: FBS schema — detailed validation
    // =====================================================================

    #[test]
    fn golden_fbs_schema_device_all_types() {
        let schema = mfg::__FBS_SCHEMA_MFGDEVICE;
        assert!(schema.contains("sn: string;"), "schema:\n{}", schema);
        assert!(schema.contains("model: uint;"), "schema:\n{}", schema);
        assert!(schema.contains("status: string;"), "schema:\n{}", schema);
        assert!(schema.contains("sku: string;"), "schema:\n{}", schema);
        assert!(schema.contains("imei: [string];"), "schema:\n{}", schema);
        assert!(schema.contains("table MfgDeviceList"), "schema:\n{}", schema);
        assert!(schema.contains("items: [MfgDevice];"), "schema:\n{}", schema);
        assert!(schema.contains("has_more: bool;"), "schema:\n{}", schema);
    }

    #[test]
    fn golden_fbs_schema_firmware_u64() {
        let schema = mfg::__FBS_SCHEMA_MFGFIRMWARE;
        assert!(schema.contains("build: ulong;"), "u64 → ulong; schema:\n{}", schema);
        assert!(schema.contains("model: uint;"), "u32 → uint; schema:\n{}", schema);
    }

    #[test]
    fn golden_fbs_schema_article_bool() {
        let schema = app_km::__FBS_SCHEMA_APPARTICLE;
        assert!(schema.contains("published: bool;"), "schema:\n{}", schema);
        assert!(schema.contains("tags: [string];"), "schema:\n{}", schema);
    }

    #[test]
    fn golden_fbs_schema_public_company() {
        let schema = public_org::__FBS_SCHEMA_PUBLICCOMPANY;
        assert!(schema.contains("table PublicCompany"), "schema:\n{}", schema);
        assert!(schema.contains("website: string;"), "schema:\n{}", schema);
        assert!(schema.contains("table PublicCompanyList"), "schema:\n{}", schema);
    }

    // =====================================================================
    // Golden: FlatBuffer encode determinism
    // =====================================================================

    #[test]
    fn golden_flatbuffer_encode_deterministic() {
        use openerp_types::IntoFlatBuffer;

        let model = mfg::MfgModel {
            code: 42,
            series_name: "test".into(),
            display_name: Some("Test Model".into()),
        };
        let buf1 = model.encode_flatbuffer();
        let buf2 = model.encode_flatbuffer();
        assert_eq!(buf1, buf2, "same input should produce same output");
    }

    #[test]
    fn golden_flatbuffer_list_encode_deterministic() {
        use openerp_types::IntoFlatBufferList;

        let items = vec![
            mfg::MfgModel { code: 1, series_name: "a".into(), display_name: None },
            mfg::MfgModel { code: 2, series_name: "b".into(), display_name: Some("B".into()) },
        ];
        let buf1 = mfg::MfgModel::encode_flatbuffer_list(&items, true);
        let buf2 = mfg::MfgModel::encode_flatbuffer_list(&items, true);
        assert_eq!(buf1, buf2, "same input should produce same output");
    }

    // =====================================================================
    // Golden: Handler check — compile-time completeness
    // =====================================================================

    // Facet with two actions for handler-check testing.
    #[facet(name = "hc", module = "test")]
    pub mod handler_check {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct RunRequest {
            pub data: String,
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct RunResponse {
            pub ok: bool,
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct ResetResponse {
            pub cleared: u32,
        }

        #[action(method = "POST", path = "/items/{id}/@run")]
        pub type Run = fn(id: String, req: RunRequest) -> RunResponse;

        #[action(method = "POST", path = "/items/{id}/@reset")]
        pub type Reset = fn(id: String) -> ResetResponse;
    }

    // Register handler implementations via impl_handler!.
    openerp_macro::impl_handler!(handler_check::Run);
    openerp_macro::impl_handler!(handler_check::Reset);

    #[test]
    fn golden_handler_traits_exist() {
        fn _assert_trait<T: handler_check::__RunHandler>() {}
        fn _assert_trait2<T: handler_check::__ResetHandler>() {}
    }

    #[test]
    fn golden_handler_registry_exists() {
        let _ = std::mem::size_of::<handler_check::__Handlers>();
    }

    #[test]
    fn golden_assert_handlers_compiles() {
        // __assert_handlers::<__Handlers>() compiles because both
        // impl_handler! calls above satisfy the marker trait bounds.
        handler_check::__assert_handlers::<handler_check::__Handlers>();
    }

    #[test]
    fn golden_no_handler_check_for_action_free_facet() {
        // public_org has no actions — no __Handlers or __assert_handlers generated.
        let ir = public_org::__facet_ir();
        assert_eq!(ir["actions"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn golden_handler_check_coexists_with_client() {
        // Handler check and client are independent features on the same facet.
        handler_check::__assert_handlers::<handler_check::__Handlers>();
        fn _check_client() {
            let _c = handler_check::HcClient::new(
                "http://localhost:8080",
                std::sync::Arc::new(openerp_client::NoAuth),
            );
        }
    }

    // =====================================================================
    // Golden: Handler check — error detection (negative cases)
    // =====================================================================

    // Runtime trait-implementation check via autoref specialization.
    // Inherent method (T: Trait) has priority over autoref fallback.
    macro_rules! does_impl {
        ($ty:ty : $trait:path) => {{
            struct __Probe<T>(std::marker::PhantomData<T>);

            impl<T: $trait> __Probe<T> {
                fn __check(&self) -> bool { true }
            }

            trait __Fallback { fn __check(&self) -> bool; }
            impl<T> __Fallback for &__Probe<T> {
                fn __check(&self) -> bool { false }
            }

            (&__Probe::<$ty>(std::marker::PhantomData)).__check()
        }};
    }

    // Facet with 3 actions — only 1 handler registered.
    #[facet(name = "partial", module = "test")]
    pub mod partial {
        #[action(method = "POST", path = "/items/@alpha")]
        pub type Alpha = fn();

        #[action(method = "POST", path = "/items/@beta")]
        pub type Beta = fn();

        #[action(method = "POST", path = "/items/@gamma")]
        pub type Gamma = fn();
    }

    // Register ONLY Alpha. Beta and Gamma are deliberately missing.
    openerp_macro::impl_handler!(partial::Alpha);

    #[test]
    fn golden_missing_handler_detected() {
        assert!(
            does_impl!(partial::__Handlers : partial::__AlphaHandler),
            "Alpha was registered via impl_handler!"
        );
        assert!(
            !does_impl!(partial::__Handlers : partial::__BetaHandler),
            "Beta was NOT registered — should be detected as missing"
        );
        assert!(
            !does_impl!(partial::__Handlers : partial::__GammaHandler),
            "Gamma was NOT registered — should be detected as missing"
        );
    }

    // Facet with actions but ZERO handlers registered.
    #[facet(name = "empty", module = "test")]
    pub mod no_handlers {
        #[action(method = "POST", path = "/items/@ping")]
        pub type Ping = fn();
    }

    #[test]
    fn golden_zero_handlers_detected() {
        assert!(
            !does_impl!(no_handlers::__Handlers : no_handlers::__PingHandler),
            "No impl_handler! was called — Ping should be missing"
        );
    }

    #[test]
    fn golden_complete_handlers_all_pass() {
        // handler_check has both Run and Reset registered.
        assert!(does_impl!(handler_check::__Handlers : handler_check::__RunHandler));
        assert!(does_impl!(handler_check::__Handlers : handler_check::__ResetHandler));
    }

    #[test]
    fn golden_assert_fails_when_incomplete() {
        // partial::__assert_handlers::<partial::__Handlers>() would NOT compile
        // because __BetaHandler and __GammaHandler are not implemented.
        // We verify this indirectly: __Handlers satisfies Alpha but not Beta/Gamma.
        let alpha = does_impl!(partial::__Handlers : partial::__AlphaHandler);
        let beta = does_impl!(partial::__Handlers : partial::__BetaHandler);
        let gamma = does_impl!(partial::__Handlers : partial::__GammaHandler);

        assert!(alpha);
        assert!(!beta);
        assert!(!gamma);
        // Since beta=false and gamma=false, the bound
        //   __Handlers: __AlphaHandler + __BetaHandler + __GammaHandler
        // is NOT satisfied, so __assert_handlers::<__Handlers>() would
        // fail to compile — exactly the behavior we want.
    }
}
