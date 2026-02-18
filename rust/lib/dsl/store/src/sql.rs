//! SqlStore trait + SqlOps CRUD operations.
//!
//! Models impl `SqlStore` to declare PK, UNIQUE, INDEX.
//! `SqlOps<T>` provides CRUD + filtered queries using SQLStore backend.
//!
//! Data is stored as JSON blob in a `data` column, with indexed fields
//! extracted into dedicated columns for efficient queries.

use openerp_core::ServiceError;
use openerp_types::Field;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

/// Trait implemented by models for SQL-backed storage.
pub trait SqlStore: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// Primary key field(s). Compound keys supported.
    const PK: &[Field];

    /// Unique constraints. Each entry is a set of fields forming a unique key.
    const UNIQUE: &[&[Field]] = &[];

    /// Index definitions. Each entry is a set of fields forming an index.
    const INDEX: &[&[Field]] = &[];

    /// Table name in SQL.
    fn table_name() -> &'static str;

    /// Extract primary key value(s) as strings (for WHERE clause).
    fn pk_values(&self) -> Vec<String>;

    /// All indexed fields (PK + UNIQUE + INDEX flattened) — used for column extraction.
    fn indexed_fields() -> Vec<&'static Field> {
        let mut fields = Vec::new();
        for f in Self::PK {
            if !fields.iter().any(|x: &&Field| x.name == f.name) {
                fields.push(f);
            }
        }
        for group in Self::UNIQUE {
            for f in *group {
                if !fields.iter().any(|x: &&Field| x.name == f.name) {
                    fields.push(f);
                }
            }
        }
        for group in Self::INDEX {
            for f in *group {
                if !fields.iter().any(|x: &&Field| x.name == f.name) {
                    fields.push(f);
                }
            }
        }
        fields
    }

    /// Called before inserting a new record.
    fn before_create(&mut self) {}

    /// Called before updating an existing record.
    fn before_update(&mut self) {}

    /// Called after a record is deleted.
    fn after_delete(&self) {}
}

/// CRUD operations for a SqlStore model.
pub struct SqlOps<T: SqlStore> {
    sql: Arc<dyn openerp_sql::SQLStore>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: SqlStore> SqlOps<T> {
    pub fn new(sql: Arc<dyn openerp_sql::SQLStore>) -> Self {
        Self {
            sql,
            _phantom: std::marker::PhantomData,
        }
    }

    fn sql_err(e: openerp_sql::SQLError) -> ServiceError {
        ServiceError::Storage(e.to_string())
    }

    /// Ensure the table exists. Call once at startup.
    pub fn ensure_table(&self) -> Result<(), ServiceError> {
        let table = T::table_name();
        let indexed = T::indexed_fields();

        // Build CREATE TABLE: indexed fields as columns + data blob.
        let mut cols = Vec::new();
        for f in &indexed {
            cols.push(format!("\"{}\" TEXT", f.name));
        }
        cols.push("data BLOB NOT NULL".to_string());

        // PK constraint.
        let pk_cols: Vec<&str> = T::PK.iter().map(|f| f.name).collect();
        let pk = format!("PRIMARY KEY ({})", pk_cols.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", "));

        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" ({}, {})",
            table,
            cols.join(", "),
            pk
        );
        self.sql
            .exec(&create_sql, &[])
            .map_err(Self::sql_err)?;

        // Unique constraints.
        for (i, group) in T::UNIQUE.iter().enumerate() {
            let ucols: Vec<String> = group.iter().map(|f| format!("\"{}\"", f.name)).collect();
            let idx_sql = format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS \"idx_{}_uq_{}\" ON \"{}\" ({})",
                table, i, table, ucols.join(", ")
            );
            self.sql.exec(&idx_sql, &[]).map_err(Self::sql_err)?;
        }

        // Regular indexes.
        for (i, group) in T::INDEX.iter().enumerate() {
            let icols: Vec<String> = group.iter().map(|f| format!("\"{}\"", f.name)).collect();
            let idx_sql = format!(
                "CREATE INDEX IF NOT EXISTS \"idx_{}_{i}\" ON \"{}\" ({})",
                table, table, icols.join(", ")
            );
            self.sql.exec(&idx_sql, &[]).map_err(Self::sql_err)?;
        }

        Ok(())
    }

    /// Get a record by primary key.
    pub fn get(&self, pk: &[&str]) -> Result<Option<T>, ServiceError> {
        let table = T::table_name();
        let pk_fields = T::PK;
        if pk.len() != pk_fields.len() {
            return Err(ServiceError::Validation("wrong number of PK values".into()));
        }

        let where_clause: Vec<String> = pk_fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("\"{}\" = ?{}", f.name, i + 1))
            .collect();
        let sql = format!(
            "SELECT data FROM \"{}\" WHERE {}",
            table,
            where_clause.join(" AND ")
        );
        let params: Vec<openerp_sql::Value> = pk
            .iter()
            .map(|v| openerp_sql::Value::Text(v.to_string()))
            .collect();

        let rows = self.sql.query(&sql, &params).map_err(Self::sql_err)?;
        if let Some(row) = rows.first() {
            if let Some(openerp_sql::Value::Blob(data)) = row.get("data") {
                let record: T = serde_json::from_slice(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                return Ok(Some(record));
            }
            if let Some(openerp_sql::Value::Text(data)) = row.get("data") {
                let record: T = serde_json::from_str(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    /// Get a record or return NotFound.
    pub fn get_or_err(&self, pk: &[&str]) -> Result<T, ServiceError> {
        self.get(pk)?.ok_or_else(|| {
            ServiceError::NotFound(format!("{} not found", T::table_name()))
        })
    }

    /// List all records.
    pub fn list(&self) -> Result<Vec<T>, ServiceError> {
        let sql = format!("SELECT data FROM \"{}\"", T::table_name());
        let rows = self.sql.query(&sql, &[]).map_err(Self::sql_err)?;
        Self::rows_to_records(&rows)
    }

    /// List records with pagination (SQL LIMIT/OFFSET).
    ///
    /// Uses SQL-native pagination — only the requested page is fetched.
    /// Fetches limit+1 rows to determine `has_more` without a COUNT query.
    pub fn list_paginated(
        &self,
        params: &openerp_core::ListParams,
    ) -> Result<openerp_core::ListResult<T>, ServiceError> {
        let fetch = params.limit + 1; // fetch one extra to detect has_more
        let sql = format!(
            "SELECT data FROM \"{}\" LIMIT ?1 OFFSET ?2",
            T::table_name()
        );
        let rows = self
            .sql
            .query(
                &sql,
                &[
                    openerp_sql::Value::Integer(fetch as i64),
                    openerp_sql::Value::Integer(params.offset as i64),
                ],
            )
            .map_err(Self::sql_err)?;

        let mut records = Self::rows_to_records(&rows)?;
        let has_more = records.len() > params.limit;
        if has_more {
            records.truncate(params.limit);
        }
        Ok(openerp_core::ListResult {
            items: records,
            has_more,
        })
    }

    /// Count all records in the table.
    pub fn count(&self) -> Result<usize, ServiceError> {
        let sql = format!("SELECT COUNT(*) AS cnt FROM \"{}\"", T::table_name());
        let rows = self.sql.query(&sql, &[]).map_err(Self::sql_err)?;
        if let Some(row) = rows.first() {
            if let Some(openerp_sql::Value::Integer(n)) = row.get("cnt") {
                return Ok(*n as usize);
            }
        }
        Ok(0)
    }

    /// Helper: convert query rows to deserialized records.
    fn rows_to_records(rows: &[openerp_sql::Row]) -> Result<Vec<T>, ServiceError> {
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(openerp_sql::Value::Blob(data)) = row.get("data") {
                let record: T = serde_json::from_slice(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            } else if let Some(openerp_sql::Value::Text(data)) = row.get("data") {
                let record: T = serde_json::from_str(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            }
        }
        Ok(records)
    }

    /// Insert a new record. Calls before_create hook.
    /// The store layer sets `createdAt` and `updatedAt`.
    pub fn save_new(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_create();

        let mut json_val: serde_json::Value = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        crate::timestamp::stamp_create(&mut json_val);
        let record: T = serde_json::from_value(json_val.clone())
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;

        let data = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        let indexed = T::indexed_fields();

        // Build INSERT.
        let mut col_names = Vec::new();
        let mut placeholders = Vec::new();
        let mut params: Vec<openerp_sql::Value> = Vec::new();

        for (i, f) in indexed.iter().enumerate() {
            col_names.push(format!("\"{}\"", f.name));
            placeholders.push(format!("?{}", i + 1));
            let val = json_val
                .get(f.name)
                .or_else(|| json_val.get(&to_camel_case(f.name)))
                .map(|v| match v {
                    serde_json::Value::String(s) => openerp_sql::Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        openerp_sql::Value::Integer(n.as_i64().unwrap_or(0))
                    }
                    serde_json::Value::Bool(b) => openerp_sql::Value::Integer(*b as i64),
                    other => openerp_sql::Value::Text(other.to_string()),
                })
                .unwrap_or(openerp_sql::Value::Null);
            params.push(val);
        }

        col_names.push("data".to_string());
        placeholders.push(format!("?{}", indexed.len() + 1));
        params.push(openerp_sql::Value::Blob(data));

        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            T::table_name(),
            col_names.join(", "),
            placeholders.join(", ")
        );

        self.sql.exec(&sql, &params).map_err(Self::sql_err)?;
        Ok(record)
    }

    /// Update an existing record with optimistic locking on `updatedAt`.
    ///
    /// Compares the incoming record's `updatedAt` with the stored value.
    /// If they don't match, returns `ServiceError::Conflict` (409).
    /// The store layer sets a fresh `updatedAt`.
    pub fn save(&self, mut record: T) -> Result<T, ServiceError> {
        let pk_values = record.pk_values();
        let pk_refs: Vec<&str> = pk_values.iter().map(|s| s.as_str()).collect();

        if let Some(existing) = self.get(&pk_refs)? {
            let existing_json = serde_json::to_value(&existing)
                .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
            let incoming_json = serde_json::to_value(&record)
                .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

            let existing_ts = existing_json.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
            let incoming_ts = incoming_json.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");

            if incoming_ts != existing_ts {
                return Err(ServiceError::Conflict(format!(
                    "updatedAt mismatch: stored {}, got {}",
                    existing_ts, incoming_ts
                )));
            }
        }

        record.before_update();

        let mut json_val = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        crate::timestamp::stamp_update(&mut json_val);
        let record: T = serde_json::from_value(json_val)
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;

        self.exec_update(&record)
    }

    /// Partially update a record using RFC 7386 JSON Merge Patch.
    ///
    /// Reads the existing record, applies the patch, and saves.
    /// Include `updatedAt` from the GET response for optimistic locking.
    /// The store layer sets a fresh `updatedAt` after merge.
    pub fn patch(&self, pk: &[&str], patch: &serde_json::Value) -> Result<T, ServiceError> {
        let existing = self.get_or_err(pk)?;
        let mut base = serde_json::to_value(&existing)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        if let Some(patch_ts) = patch.get("updatedAt").and_then(|v| v.as_str()) {
            let base_ts = base.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
            if patch_ts != base_ts {
                return Err(ServiceError::Conflict(format!(
                    "updatedAt mismatch: stored {}, got {}",
                    base_ts, patch_ts
                )));
            }
        }

        openerp_core::merge_patch(&mut base, patch);

        let mut record: T = serde_json::from_value(base)
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
        record.before_update();

        let mut json_val = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;
        crate::timestamp::stamp_update(&mut json_val);
        let record: T = serde_json::from_value(json_val)
            .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;

        self.exec_update(&record)
    }

    /// Execute an UPDATE statement for the given record.
    /// Shared by save() and patch() to avoid SQL building duplication.
    fn exec_update(&self, record: &T) -> Result<T, ServiceError> {
        let data = serde_json::to_vec(record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        let indexed = T::indexed_fields();
        let json_val: serde_json::Value = serde_json::to_value(record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        let pk_fields = T::PK;
        let pk_values = record.pk_values();

        let mut set_clauses = Vec::new();
        let mut params: Vec<openerp_sql::Value> = Vec::new();
        let mut idx = 1;

        for f in &indexed {
            if pk_fields.iter().any(|pk| pk.name == f.name) {
                continue; // Don't update PK columns.
            }
            set_clauses.push(format!("\"{}\" = ?{}", f.name, idx));
            let val = json_val
                .get(f.name)
                .or_else(|| json_val.get(&to_camel_case(f.name)))
                .map(|v| match v {
                    serde_json::Value::String(s) => openerp_sql::Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        openerp_sql::Value::Integer(n.as_i64().unwrap_or(0))
                    }
                    serde_json::Value::Bool(b) => openerp_sql::Value::Integer(*b as i64),
                    other => openerp_sql::Value::Text(other.to_string()),
                })
                .unwrap_or(openerp_sql::Value::Null);
            params.push(val);
            idx += 1;
        }

        set_clauses.push(format!("data = ?{}", idx));
        params.push(openerp_sql::Value::Blob(data));
        idx += 1;

        let where_clause: Vec<String> = pk_fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                params.push(openerp_sql::Value::Text(pk_values[i].clone()));
                format!("\"{}\" = ?{}", f.name, idx + i)
            })
            .collect();

        let sql = format!(
            "UPDATE \"{}\" SET {} WHERE {}",
            T::table_name(),
            set_clauses.join(", "),
            where_clause.join(" AND ")
        );

        self.sql.exec(&sql, &params).map_err(Self::sql_err)?;
        Ok(record.clone())
    }

    /// Delete a record by primary key.
    pub fn delete(&self, pk: &[&str]) -> Result<(), ServiceError> {
        let record = self.get_or_err(pk)?;
        let pk_fields = T::PK;

        let where_clause: Vec<String> = pk_fields
            .iter()
            .enumerate()
            .map(|(i, f)| format!("\"{}\" = ?{}", f.name, i + 1))
            .collect();
        let params: Vec<openerp_sql::Value> = pk
            .iter()
            .map(|v| openerp_sql::Value::Text(v.to_string()))
            .collect();

        let sql = format!(
            "DELETE FROM \"{}\" WHERE {}",
            T::table_name(),
            where_clause.join(" AND ")
        );

        self.sql.exec(&sql, &params).map_err(Self::sql_err)?;
        record.after_delete();
        Ok(())
    }

    /// Query by a single indexed field.
    pub fn find_by(&self, field: &Field, value: &str) -> Result<Vec<T>, ServiceError> {
        let sql = format!(
            "SELECT data FROM \"{}\" WHERE \"{}\" = ?1",
            T::table_name(),
            field.name
        );
        let rows = self
            .sql
            .query(&sql, &[openerp_sql::Value::Text(value.to_string())])
            .map_err(Self::sql_err)?;

        let mut records = Vec::new();
        for row in &rows {
            if let Some(openerp_sql::Value::Blob(data)) = row.get("data") {
                let record: T = serde_json::from_slice(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            } else if let Some(openerp_sql::Value::Text(data)) = row.get("data") {
                let record: T = serde_json::from_str(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            }
        }
        Ok(records)
    }
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Device {
        sn: String,
        model: u32,
        status: String,
        description: Option<String>,
    }

    impl SqlStore for Device {
        const PK: &[Field] = &[Field::new("sn", "String", "text")];
        const INDEX: &[&[Field]] = &[
            &[Field::new("model", "u32", "number")],
            &[Field::new("status", "String", "text")],
        ];

        fn table_name() -> &'static str {
            "devices"
        }

        fn pk_values(&self) -> Vec<String> {
            vec![self.sn.clone()]
        }
    }

    fn make_ops() -> (SqlOps<Device>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let sql: Arc<dyn openerp_sql::SQLStore> =
            Arc::new(openerp_sql::SqliteStore::open(&dir.path().join("test.db")).unwrap());
        let ops = SqlOps::new(sql);
        ops.ensure_table().unwrap();
        (ops, dir)
    }

    #[test]
    fn sql_crud_lifecycle() {
        let (ops, _dir) = make_ops();

        let device = Device {
            sn: "SN001".into(),
            model: 42,
            status: "active".into(),
            description: Some("Test device".into()),
        };
        let created = ops.save_new(device).unwrap();
        assert_eq!(created.sn, "SN001");

        let fetched = ops.get_or_err(&["SN001"]).unwrap();
        assert_eq!(fetched.model, 42);

        let all = ops.list().unwrap();
        assert_eq!(all.len(), 1);

        // Find by indexed field.
        let by_model = ops.find_by(&Device::PK[0], "SN001").unwrap();
        assert_eq!(by_model.len(), 1);

        let by_status = ops.find_by(
            &Field::new("status", "String", "text"),
            "active",
        ).unwrap();
        assert_eq!(by_status.len(), 1);

        // Delete.
        ops.delete(&["SN001"]).unwrap();
        assert!(ops.get(&["SN001"]).unwrap().is_none());
    }

    #[test]
    fn sql_update_existing() {
        let (ops, _dir) = make_ops();

        let device = Device {
            sn: "UPD001".into(),
            model: 10,
            status: "provisioned".into(),
            description: Some("Before update".into()),
        };
        ops.save_new(device).unwrap();

        // Update.
        let mut d = ops.get_or_err(&["UPD001"]).unwrap();
        d.status = "active".into();
        d.description = Some("After update".into());
        let updated = ops.save(d).unwrap();
        assert_eq!(updated.status, "active");

        // Verify.
        let fetched = ops.get_or_err(&["UPD001"]).unwrap();
        assert_eq!(fetched.status, "active");
        assert_eq!(fetched.description, Some("After update".into()));

        // Should still be 1 record.
        assert_eq!(ops.list().unwrap().len(), 1);
    }

    #[test]
    fn sql_unique_violation() {
        let (ops, _dir) = make_ops();

        let d1 = Device { sn: "DUP001".into(), model: 1, status: "a".into(), description: None };
        ops.save_new(d1).unwrap();

        let d2 = Device { sn: "DUP001".into(), model: 2, status: "b".into(), description: None };
        let err = ops.save_new(d2).unwrap_err();
        assert!(err.to_string().contains("UNIQUE") || err.to_string().contains("already exists"),
            "Duplicate PK should fail: {}", err);
    }

    #[test]
    fn sql_get_nonexistent() {
        let (ops, _dir) = make_ops();
        assert!(ops.get(&["ghost"]).unwrap().is_none());
        let err = ops.get_or_err(&["ghost"]).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    // Compound PK test.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Firmware {
        model: u32,
        semver: String,
        build: u64,
    }

    impl SqlStore for Firmware {
        const PK: &[Field] = &[
            Field::new("model", "u32", "number"),
            Field::new("semver", "String", "text"),
        ];

        fn table_name() -> &'static str {
            "firmware"
        }

        fn pk_values(&self) -> Vec<String> {
            vec![self.model.to_string(), self.semver.clone()]
        }
    }

    #[test]
    fn compound_pk() {
        let dir = tempfile::tempdir().unwrap();
        let sql: Arc<dyn openerp_sql::SQLStore> =
            Arc::new(openerp_sql::SqliteStore::open(&dir.path().join("test2.db")).unwrap());
        let ops = SqlOps::new(sql);
        ops.ensure_table().unwrap();

        let fw = Firmware {
            model: 100,
            semver: "1.0.0".into(),
            build: 42,
        };
        ops.save_new(fw).unwrap();

        let fetched = ops.get_or_err(&["100", "1.0.0"]).unwrap();
        assert_eq!(fetched.build, 42);

        ops.delete(&["100", "1.0.0"]).unwrap();
        assert!(ops.get(&["100", "1.0.0"]).unwrap().is_none());
    }

    #[test]
    fn sql_list_paginated() {
        let (ops, _dir) = make_ops();

        // Insert 5 devices.
        for i in 0..5 {
            let d = Device {
                sn: format!("PG{:03}", i),
                model: i,
                status: "active".into(),
                description: None,
            };
            ops.save_new(d).unwrap();
        }

        // Page 1: limit=2.
        let params = openerp_core::ListParams { limit: 2, offset: 0, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);

        // Page 2.
        let params = openerp_core::ListParams { limit: 2, offset: 2, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 2);
        assert!(result.has_more);

        // Page 3 (last).
        let params = openerp_core::ListParams { limit: 2, offset: 4, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 1);
        assert!(!result.has_more);

        // Beyond range.
        let params = openerp_core::ListParams { limit: 10, offset: 100, ..Default::default() };
        let result = ops.list_paginated(&params).unwrap();
        assert_eq!(result.items.len(), 0);
        assert!(!result.has_more);
    }

    #[test]
    fn sql_count() {
        let (ops, _dir) = make_ops();
        assert_eq!(ops.count().unwrap(), 0);

        for i in 0..3 {
            let d = Device {
                sn: format!("CNT{}", i),
                model: i,
                status: "a".into(),
                description: None,
            };
            ops.save_new(d).unwrap();
        }
        assert_eq!(ops.count().unwrap(), 3);

        ops.delete(&["CNT1"]).unwrap();
        assert_eq!(ops.count().unwrap(), 2);
    }
}
