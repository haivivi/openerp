use serde::Serialize;

use crate::model::{Device, Model, Firmware};
use openerp_core::ServiceError;
use super::PmsService;

/// Aggregated device information — joins device with related entities.
///
/// `secret` is deliberately omitted to avoid leaking authentication
/// credentials through the SN-based lookup endpoint (SN is printed on
/// physical devices). The by-secret endpoint returns secret implicitly
/// because the caller already possesses it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub sn: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<Firmware>,
    pub licenses: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_at: Option<String>,
}

impl PmsService {
    fn build_device_info(&self, device: &Device) -> DeviceInfo {
        let model = self.get_model(device.model).ok();
        let firmware = self.list_firmwares_for_model(device.model)
            .ok()
            .and_then(|fws| fws.into_iter().next());

        let status_str = serde_json::to_value(&device.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "PENDING".into());

        DeviceInfo {
            sn: device.sn.clone(),
            status: status_str,
            model,
            firmware,
            licenses: device.licenses.clone(),
            create_at: device.create_at.clone(),
            update_at: device.update_at.clone(),
        }
    }

    pub fn get_device_info_by_sn(&self, sn: &str) -> Result<DeviceInfo, ServiceError> {
        let device = self.get_device(sn)?;
        Ok(self.build_device_info(&device))
    }

    pub fn get_device_info_by_secret(&self, secret: &str) -> Result<DeviceInfo, ServiceError> {
        let device = self.get_device_by_secret(secret)?;
        Ok(self.build_device_info(&device))
    }
}
