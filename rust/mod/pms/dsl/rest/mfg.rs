//! "mfg" facet — API surface for the Manufacturing App.
//!
//! Factory floor operators use this facet for device provisioning,
//! batch management, firmware flashing, and license assignment.
//!
//! Defined with `#[facet]` macro — generates serde types, metadata, and client.
//! Handlers remain hand-written in `src/handlers/mfg/`.

#[openerp_macro::facet(name = "mfg", module = "pms")]
pub mod mfg {
    // ── Resource projections ────────────────────────────────────────

    /// Product model — code, series name, display name.
    #[resource(path = "/models", pk = "code")]
    pub struct MfgModel {
        pub code: u32,
        pub series_name: String,
        pub display_name: Option<String>,
    }

    /// Production batch — progress tracking.
    #[resource(path = "/batches", pk = "id")]
    pub struct MfgBatch {
        pub id: String,
        pub model: u32,
        pub quantity: u32,
        pub provisioned_count: u32,
        pub status: String,
        pub display_name: Option<String>,
    }

    /// Device — SN, model, status. No secrets exposed.
    #[resource(path = "/devices", pk = "sn")]
    pub struct MfgDevice {
        pub sn: String,
        pub model: u32,
        pub status: String,
        pub sku: Option<String>,
        pub imei: Vec<String>,
        pub licenses: Vec<String>,
        pub display_name: Option<String>,
    }

    /// Firmware — version info for flashing.
    #[resource(path = "/firmwares", pk = "id")]
    pub struct MfgFirmware {
        pub id: String,
        pub model: u32,
        pub semver: String,
        pub build: u64,
        pub status: String,
        pub display_name: Option<String>,
    }

    // ── Action request/response types ───────────────────────────────

    /// Request body for batch provisioning.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ProvisionRequest {
        /// Number of devices to provision (defaults to remaining).
        pub count: Option<u32>,
    }

    /// Response from batch provisioning.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ProvisionResponse {
        pub batch_id: String,
        pub provisioned: u32,
        pub devices: Vec<String>,
    }

    /// Response from device activation.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ActivateResponse {
        pub sn: String,
        pub status: String,
    }

    // ── Action signatures ───────────────────────────────────────────

    /// Provision devices for a batch.
    #[action(method = "POST", path = "/batches/{id}/@provision")]
    pub type Provision = fn(id: String, req: ProvisionRequest) -> ProvisionResponse;

    /// Activate a provisioned device.
    #[action(method = "POST", path = "/devices/{sn}/@activate")]
    pub type Activate = fn(sn: String) -> ActivateResponse;
}
