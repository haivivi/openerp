use openerp_core::{ListParams, ListResult, ServiceError, new_id, now_rfc3339};
use openerp_sql::Value;

use crate::model::{Firmware, FirmwareFile, FirmwareStatus};
use super::PmsService;

/// Parameters for creating a new firmware record.
pub struct CreateFirmwareInput {
    pub model: u32,
    pub semver: String,
    pub build: u64,
    pub files: Vec<FirmwareFile>,
    pub release_notes: Option<String>,
}

/// Query filters for listing firmwares.
#[derive(Debug, Default)]
pub struct FirmwareFilters {
    pub model: Option<u32>,
    pub status: Option<String>,
}

impl PmsService {
    pub fn create_firmware(&self, input: CreateFirmwareInput) -> Result<Firmware, ServiceError> {
        let id = Firmware::composite_key(input.model, &input.semver);
        let now = now_rfc3339();

        let record = Firmware {
            id: id.clone(),
            model: input.model,
            semver: input.semver.clone(),
            build: input.build,
            status: FirmwareStatus::Draft,
            files: input.files,
            release_notes: input.release_notes,
            display_name: None,
            description: None,
            data: None,
            create_at: Some(now.clone()),
            update_at: Some(now.clone()),
        };

        self.insert_record(
            "firmwares",
            &id,
            &record,
            &[
                ("model", Value::Integer(input.model as i64)),
                ("semver", Value::Text(input.semver)),
                ("build", Value::Integer(input.build as i64)),
                ("status", Value::Text("DRAFT".into())),
                ("create_at", Value::Text(now.clone())),
                ("update_at", Value::Text(now)),
            ],
        )?;

        Ok(record)
    }

    pub fn get_firmware(&self, model: u32, semver: &str) -> Result<Firmware, ServiceError> {
        let id = Firmware::composite_key(model, semver);
        self.get_record("firmwares", &id)
    }

    pub fn list_firmwares(
        &self,
        params: &ListParams,
        filters: &FirmwareFilters,
    ) -> Result<ListResult<Firmware>, ServiceError> {
        let limit = params.limit.min(500);
        let mut f: Vec<(&str, Value)> = Vec::new();
        if let Some(model) = filters.model {
            f.push(("model", Value::Integer(model as i64)));
        }
        if let Some(ref st) = filters.status {
            f.push(("status", Value::Text(st.clone())));
        }
        self.list_records("firmwares", &f, limit, params.offset, true)
    }

    pub fn update_firmware(
        &self,
        model: u32,
        semver: &str,
        patch: serde_json::Value,
    ) -> Result<Firmware, ServiceError> {
        let id = Firmware::composite_key(model, semver);
        let current: Firmware = self.get_record("firmwares", &id)?;
        let updated: Firmware = Self::apply_patch(&current, patch, &["model", "semver"])?;

        let status_str = serde_json::to_value(&updated.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "DRAFT".into());

        self.update_record(
            "firmwares",
            &id,
            &updated,
            &[
                ("model", Value::Integer(updated.model as i64)),
                ("semver", Value::Text(updated.semver.clone())),
                ("build", Value::Integer(updated.build as i64)),
                ("status", Value::Text(status_str)),
                ("update_at", Value::Text(updated.update_at.clone().unwrap_or_default())),
            ],
        )?;

        Ok(updated)
    }

    pub fn delete_firmware(&self, model: u32, semver: &str) -> Result<(), ServiceError> {
        let id = Firmware::composite_key(model, semver);
        self.delete_record("firmwares", &id)
    }

    /// List published firmwares for a model, ordered by build desc.
    pub(crate) fn list_firmwares_for_model(&self, model: u32) -> Result<Vec<Firmware>, ServiceError> {
        let sql = "SELECT data FROM firmwares WHERE model = ?1 AND status = 'PUBLISHED' ORDER BY build DESC LIMIT 5";
        let rows = self.sql
            .query(sql, &[Value::Integer(model as i64)])
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let mut items = Vec::new();
        for row in &rows {
            if let Some(data) = row.get_str("data") {
                if let Ok(fw) = serde_json::from_str::<Firmware>(data) {
                    items.push(fw);
                }
            }
        }
        Ok(items)
    }
}
