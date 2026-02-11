use std::collections::HashMap;

use openerp_core::{ListParams, ListResult, ServiceError, new_id, now_rfc3339};
use sql::Value;

use crate::model::{Batch, BatchStatus, Device, DeviceStatus};
use crate::sn::{default_config, encode_sn, EncodeInput};
use super::PmsService;

pub struct CreateBatchInput {
    pub name: String,
    pub model: u32,
    pub quantity: u32,
    pub description: Option<String>,
}

#[derive(Debug, Default)]
pub struct BatchFilters {
    pub model: Option<u32>,
    pub status: Option<String>,
}

#[derive(Debug, Default)]
pub struct DeviceFilters {
    pub model: Option<u32>,
    pub batch_id: Option<String>,
    pub status: Option<String>,
}

impl PmsService {
    // ── Batch ──

    pub fn create_batch(&self, input: CreateBatchInput) -> Result<Batch, ServiceError> {
        // Validate model exists
        let _model = self.get_model(input.model)?;

        let id = new_id();
        let now = now_rfc3339();
        let record = Batch {
            id: id.clone(),
            name: input.name.clone(),
            model: input.model,
            quantity: input.quantity,
            provisioned_count: 0,
            status: BatchStatus::Draft,
            display_name: None,
            description: input.description,
            data: None,
            create_at: Some(now.clone()),
            update_at: Some(now.clone()),
        };

        self.insert_record("batches", &id, &record, &[
            ("name", Value::Text(input.name)),
            ("model", Value::Integer(input.model as i64)),
            ("status", Value::Text("DRAFT".into())),
            ("create_at", Value::Text(now.clone())),
            ("update_at", Value::Text(now)),
        ])?;

        Ok(record)
    }

    pub fn get_batch(&self, id: &str) -> Result<Batch, ServiceError> {
        self.get_record("batches", id)
    }

    pub fn list_batches(
        &self,
        params: &ListParams,
        filters: &BatchFilters,
    ) -> Result<ListResult<Batch>, ServiceError> {
        let limit = params.limit.min(500);
        let mut f: Vec<(&str, Value)> = Vec::new();
        if let Some(m) = filters.model {
            f.push(("model", Value::Integer(m as i64)));
        }
        if let Some(ref s) = filters.status {
            f.push(("status", Value::Text(s.clone())));
        }
        self.list_records("batches", &f, limit, params.offset, true)
    }

    pub fn update_batch(
        &self,
        id: &str,
        patch: serde_json::Value,
    ) -> Result<Batch, ServiceError> {
        let current: Batch = self.get_record("batches", id)?;
        let updated: Batch = Self::apply_patch(&current, patch)?;

        let status_str = serde_json::to_value(&updated.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "DRAFT".into());

        self.update_record("batches", id, &updated, &[
            ("name", Value::Text(updated.name.clone())),
            ("model", Value::Integer(updated.model as i64)),
            ("status", Value::Text(status_str)),
            ("update_at", Value::Text(updated.update_at.clone().unwrap_or_default())),
        ])?;

        Ok(updated)
    }

    pub fn delete_batch(&self, id: &str) -> Result<(), ServiceError> {
        self.delete_record("batches", id)
    }

    /// Provision a batch: generate devices with SNs and allocate licenses.
    pub fn provision_batch(&self, batch_id: &str) -> Result<Batch, ServiceError> {
        let mut batch: Batch = self.get_batch(batch_id)?;
        if batch.status != BatchStatus::Draft && batch.status != BatchStatus::Provisioning {
            return Err(ServiceError::Validation(format!(
                "batch {} cannot be provisioned in {:?} status",
                batch_id, batch.status
            )));
        }

        let sn_config = self.get_sn_config().unwrap_or_else(|_| default_config());
        let now_utc = chrono::Utc::now();
        let year = now_utc.format("%Y").to_string().parse::<u32>().unwrap_or(2025);
        let week = now_utc.format("%W").to_string().parse::<u32>().unwrap_or(1);

        // Mark as provisioning
        batch.status = BatchStatus::Provisioning;
        let now = now_rfc3339();
        batch.update_at = Some(now.clone());
        self.update_record("batches", batch_id, &batch, &[
            ("status", Value::Text("PROVISIONING".into())),
            ("update_at", Value::Text(now)),
        ])?;

        let remaining = batch.quantity - batch.provisioned_count;
        for _ in 0..remaining {
            let mut dimensions = HashMap::new();
            dimensions.insert("manufacturer".into(), 0u32);
            dimensions.insert("channel".into(), 0u32);

            let encoded = encode_sn(&sn_config, &EncodeInput {
                model_no: batch.model,
                dimensions,
                timestamp: Some((year, week)),
            }).map_err(|e| ServiceError::Internal(format!("SN encode failed: {}", e)))?;

            let sn = encoded.formatted;
            let secret = new_id();
            let now = now_rfc3339();

            let device = Device {
                sn: sn.clone(),
                secret: secret.clone(),
                model: batch.model,
                status: DeviceStatus::Provisioned,
                sku: None,
                imei: vec![],
                licenses: vec![],
                display_name: None,
                description: None,
                data: None,
                create_at: Some(now.clone()),
                update_at: Some(now.clone()),
            };

            self.insert_record("devices", &sn, &device, &[
                ("sn", Value::Text(sn.clone())),
                ("secret", Value::Text(secret)),
                ("model", Value::Integer(batch.model as i64)),
                ("batch_id", Value::Text(batch_id.to_string())),
                ("status", Value::Text("PROVISIONED".into())),
                ("create_at", Value::Text(now.clone())),
                ("update_at", Value::Text(now)),
            ])?;

            // Index device for search
            let mut doc = HashMap::new();
            doc.insert("sn".into(), sn.clone());
            doc.insert("model".into(), batch.model.to_string());
            let _ = self.search.index("devices", &sn, doc);

            batch.provisioned_count += 1;
        }

        // Mark as completed
        batch.status = BatchStatus::Completed;
        let now = now_rfc3339();
        batch.update_at = Some(now.clone());
        self.update_record("batches", batch_id, &batch, &[
            ("status", Value::Text("COMPLETED".into())),
            ("update_at", Value::Text(now)),
        ])?;

        Ok(batch)
    }

    // ── Device (read-only, PK = sn) ──

    pub fn get_device(&self, sn: &str) -> Result<Device, ServiceError> {
        self.get_record("devices", sn)
    }

    pub fn list_devices(
        &self,
        params: &ListParams,
        filters: &DeviceFilters,
    ) -> Result<ListResult<Device>, ServiceError> {
        let limit = params.limit.min(500);
        let mut f: Vec<(&str, Value)> = Vec::new();
        if let Some(m) = filters.model {
            f.push(("model", Value::Integer(m as i64)));
        }
        if let Some(ref v) = filters.batch_id {
            f.push(("batch_id", Value::Text(v.clone())));
        }
        if let Some(ref v) = filters.status {
            f.push(("status", Value::Text(v.clone())));
        }
        self.list_records("devices", &f, limit, params.offset, true)
    }

    pub fn search_devices(&self, query: &str, limit: usize) -> Result<Vec<Device>, ServiceError> {
        let results = self.search.search("devices", query, limit)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let mut devices = Vec::new();
        for r in results {
            if let Ok(d) = self.get_device(&r.id) {
                devices.push(d);
            }
        }
        Ok(devices)
    }

    pub fn get_device_by_secret(&self, secret: &str) -> Result<Device, ServiceError> {
        let sql = "SELECT data FROM devices WHERE secret = ?1";
        let rows = self.sql
            .query(sql, &[Value::Text(secret.to_string())])
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let row = rows.first()
            .ok_or_else(|| ServiceError::NotFound("device not found by secret".into()))?;
        let data = row.get_str("data")
            .ok_or_else(|| ServiceError::Internal("missing data".into()))?;
        serde_json::from_str(data).map_err(|e| ServiceError::Internal(e.to_string()))
    }
}
