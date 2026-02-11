use openerp_core::{ListParams, ListResult, ServiceError, new_id, now_rfc3339};
use openerp_sql::Value;

use crate::model::{License, LicenseImport, LicenseSource, LicenseStatus};
use super::PmsService;

/// Query filters for listing licenses.
#[derive(Debug, Default)]
pub struct LicenseFilters {
    pub license_type: Option<String>,
    pub status: Option<String>,
    pub sn: Option<String>,
    pub import_id: Option<String>,
}

impl PmsService {
    // ── LicenseImport ──

    pub fn create_license_import(
        &self,
        license_type: String,
        source: LicenseSource,
        name: String,
    ) -> Result<LicenseImport, ServiceError> {
        let id = new_id();
        let now = now_rfc3339();

        let source_str = serde_json::to_value(&source)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "IMPORT".into());

        let record = LicenseImport {
            id: id.clone(),
            license_type: license_type.clone(),
            source,
            name: name.clone(),
            count: 0,
            allocated_count: 0,
            display_name: None,
            description: None,
            data: None,
            create_at: Some(now.clone()),
            update_at: Some(now.clone()),
        };

        self.insert_record("license_imports", &id, &record, &[
            ("license_type", Value::Text(license_type)),
            ("source", Value::Text(source_str)),
            ("name", Value::Text(name)),
            ("create_at", Value::Text(now.clone())),
            ("update_at", Value::Text(now)),
        ])?;

        Ok(record)
    }

    pub fn get_license_import(&self, id: &str) -> Result<LicenseImport, ServiceError> {
        self.get_record("license_imports", id)
    }

    pub fn list_license_imports(
        &self,
        params: &ListParams,
    ) -> Result<ListResult<LicenseImport>, ServiceError> {
        let limit = params.limit.min(500);
        self.list_records("license_imports", &[], limit, params.offset, true)
    }

    pub fn delete_license_import(&self, id: &str) -> Result<(), ServiceError> {
        self.delete_record("license_imports", id)
    }

    /// Execute import: create licenses from provided list.
    pub fn execute_import(
        &self,
        import_id: &str,
        entries: Vec<(String, Option<String>)>, // (number, optional metadata)
    ) -> Result<Vec<License>, ServiceError> {
        let import = self.get_license_import(import_id)?;
        let now = now_rfc3339();
        let mut created = Vec::new();

        for (number, data) in entries {
            let id = License::composite_key(&import.license_type, &number);

            let record = License {
                id: id.clone(),
                license_type: import.license_type.clone(),
                number: number.clone(),
                source: import.source,
                sn: None,
                import_id: Some(import_id.to_string()),
                status: LicenseStatus::Available,
                display_name: None,
                description: None,
                data,
                create_at: Some(now.clone()),
                update_at: Some(now.clone()),
            };

            self.insert_record("licenses", &id, &record, &[
                ("license_type", Value::Text(import.license_type.clone())),
                ("number", Value::Text(number)),
                ("import_id", Value::Text(import_id.to_string())),
                ("status", Value::Text("AVAILABLE".into())),
                ("create_at", Value::Text(now.clone())),
                ("update_at", Value::Text(now.clone())),
            ])?;

            created.push(record);
        }

        // Update import counts
        self.refresh_import_counts(import_id)?;

        Ok(created)
    }

    /// Execute generate: create licenses with auto-generated numbers.
    pub fn execute_generate(
        &self,
        import_id: &str,
        prefix: &str,
        count: u64,
    ) -> Result<Vec<License>, ServiceError> {
        let import = self.get_license_import(import_id)?;
        let now = now_rfc3339();
        let mut created = Vec::new();

        for i in 0..count {
            let number = format!("{}{:06}", prefix, i + 1);
            let id = License::composite_key(&import.license_type, &number);

            let record = License {
                id: id.clone(),
                license_type: import.license_type.clone(),
                number: number.clone(),
                source: LicenseSource::Generate,
                sn: None,
                import_id: Some(import_id.to_string()),
                status: LicenseStatus::Available,
                display_name: None,
                description: None,
                data: None,
                create_at: Some(now.clone()),
                update_at: Some(now.clone()),
            };

            self.insert_record("licenses", &id, &record, &[
                ("license_type", Value::Text(import.license_type.clone())),
                ("number", Value::Text(number)),
                ("import_id", Value::Text(import_id.to_string())),
                ("status", Value::Text("AVAILABLE".into())),
                ("create_at", Value::Text(now.clone())),
                ("update_at", Value::Text(now.clone())),
            ])?;

            created.push(record);
        }

        self.refresh_import_counts(import_id)?;
        Ok(created)
    }

    // ── License ──

    pub fn get_license(&self, license_type: &str, number: &str) -> Result<License, ServiceError> {
        let id = License::composite_key(license_type, number);
        self.get_record("licenses", &id)
    }

    pub fn list_licenses(
        &self,
        params: &ListParams,
        filters: &LicenseFilters,
    ) -> Result<ListResult<License>, ServiceError> {
        let limit = params.limit.min(500);
        let mut f: Vec<(&str, Value)> = Vec::new();
        if let Some(ref t) = filters.license_type {
            f.push(("license_type", Value::Text(t.clone())));
        }
        if let Some(ref st) = filters.status {
            f.push(("status", Value::Text(st.clone())));
        }
        if let Some(ref sn) = filters.sn {
            f.push(("sn", Value::Text(sn.clone())));
        }
        if let Some(ref iid) = filters.import_id {
            f.push(("import_id", Value::Text(iid.clone())));
        }
        self.list_records("licenses", &f, limit, params.offset, true)
    }

    pub fn count_licenses(&self, filters: &LicenseFilters) -> Result<i64, ServiceError> {
        let mut f: Vec<(&str, Value)> = Vec::new();
        if let Some(ref t) = filters.license_type {
            f.push(("license_type", Value::Text(t.clone())));
        }
        if let Some(ref st) = filters.status {
            f.push(("status", Value::Text(st.clone())));
        }
        self.count_records("licenses", &f)
    }

    pub fn update_license(
        &self,
        license_type: &str,
        number: &str,
        patch: serde_json::Value,
    ) -> Result<License, ServiceError> {
        let id = License::composite_key(license_type, number);
        let current: License = self.get_record("licenses", &id)?;
        let updated: License = Self::apply_patch(&current, patch)?;

        let status_str = serde_json::to_value(&updated.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "AVAILABLE".into());

        self.update_record("licenses", &id, &updated, &[
            ("status", Value::Text(status_str)),
            ("sn", updated.sn.as_ref()
                .map(|s| Value::Text(s.clone()))
                .unwrap_or(Value::Null)),
            ("update_at", Value::Text(updated.update_at.clone().unwrap_or_default())),
        ])?;

        Ok(updated)
    }

    pub fn delete_license(&self, license_type: &str, number: &str) -> Result<(), ServiceError> {
        let id = License::composite_key(license_type, number);
        let lic: License = self.get_record("licenses", &id)?;
        self.delete_record("licenses", &id)?;
        if let Some(ref iid) = lic.import_id {
            let _ = self.refresh_import_counts(iid);
        }
        Ok(())
    }

    /// Allocate an available license of a given type to a device.
    pub(crate) fn allocate_license(
        &self,
        license_type: &str,
        device_sn: &str,
    ) -> Result<License, ServiceError> {
        let sql = "SELECT data FROM licenses WHERE license_type = ?1 AND status = 'AVAILABLE' LIMIT 1";
        let rows = self.sql
            .query(sql, &[Value::Text(license_type.to_string())])
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let row = rows.first()
            .ok_or_else(|| ServiceError::Validation(format!(
                "no available licenses of type {}", license_type
            )))?;
        let data = row.get_str("data")
            .ok_or_else(|| ServiceError::Internal("missing data".into()))?;
        let mut lic: License = serde_json::from_str(data)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let now = now_rfc3339();
        lic.sn = Some(device_sn.to_string());
        lic.status = LicenseStatus::Allocated;
        lic.update_at = Some(now.clone());

        self.update_record("licenses", &lic.id, &lic, &[
            ("status", Value::Text("ALLOCATED".into())),
            ("sn", Value::Text(device_sn.to_string())),
            ("update_at", Value::Text(now)),
        ])?;

        if let Some(ref iid) = lic.import_id {
            let _ = self.refresh_import_counts(iid);
        }

        Ok(lic)
    }

    /// Recalculate and update import counts.
    fn refresh_import_counts(&self, import_id: &str) -> Result<(), ServiceError> {
        let total_sql = "SELECT COUNT(*) as cnt FROM licenses WHERE import_id = ?1";
        let alloc_sql = "SELECT COUNT(*) as cnt FROM licenses WHERE import_id = ?1 AND status = 'ALLOCATED'";

        let params = &[Value::Text(import_id.to_string())];

        let total = self.sql.query(total_sql, params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?
            .first()
            .and_then(|r| r.get_i64("cnt"))
            .unwrap_or(0);

        let allocated = self.sql.query(alloc_sql, params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?
            .first()
            .and_then(|r| r.get_i64("cnt"))
            .unwrap_or(0);

        let mut import: LicenseImport = self.get_record("license_imports", import_id)?;
        let now = now_rfc3339();
        import.count = total as u64;
        import.allocated_count = allocated as u64;
        import.update_at = Some(now.clone());

        self.update_record("license_imports", import_id, &import, &[
            ("update_at", Value::Text(now)),
        ])?;

        Ok(())
    }
}
