use openerp_core::{ListParams, ListResult, ServiceError, new_id, now_rfc3339};
use openerp_sql::Value;

use crate::model::Model;
use super::PmsService;

impl PmsService {
    pub fn create_model(
        &self,
        code: u32,
        series_name: String,
        display_name: Option<String>,
        description: Option<String>,
        data: Option<String>,
    ) -> Result<Model, ServiceError> {
        let now = now_rfc3339();
        let id = code.to_string();

        let record = Model {
            code,
            series_name: series_name.clone(),
            display_name,
            description,
            data,
            create_at: Some(now.clone()),
            update_at: Some(now.clone()),
        };

        self.insert_record(
            "models",
            &id,
            &record,
            &[
                ("code", Value::Integer(code as i64)),
                ("series_name", Value::Text(series_name)),
                ("create_at", Value::Text(now.clone())),
                ("update_at", Value::Text(now)),
            ],
        )?;

        Ok(record)
    }

    pub fn get_model(&self, code: u32) -> Result<Model, ServiceError> {
        self.get_record("models", &code.to_string())
    }

    pub fn list_models(&self, params: &ListParams) -> Result<ListResult<Model>, ServiceError> {
        let limit = params.limit.min(500);
        self.list_records("models", &[], limit, params.offset, true)
    }

    pub fn count_models(&self) -> Result<i64, ServiceError> {
        self.count_records("models", &[])
    }

    pub fn update_model(
        &self,
        code: u32,
        patch: serde_json::Value,
    ) -> Result<Model, ServiceError> {
        let id = code.to_string();
        let current: Model = self.get_record("models", &id)?;
        let updated: Model = Self::apply_patch(&current, patch)?;

        self.update_record(
            "models",
            &id,
            &updated,
            &[
                ("code", Value::Integer(updated.code as i64)),
                ("series_name", Value::Text(updated.series_name.clone())),
                ("update_at", Value::Text(updated.update_at.clone().unwrap_or_default())),
            ],
        )?;

        Ok(updated)
    }

    pub fn delete_model(&self, code: u32) -> Result<(), ServiceError> {
        self.delete_record("models", &code.to_string())
    }
}
