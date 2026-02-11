pub mod schema;
pub mod model;
pub mod firmware;
pub mod device;
pub mod license;
pub mod sn_service;
pub mod device_info;

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::Serialize;

use openerp_core::{ListParams, ListResult, ServiceError, merge_patch, new_id, now_rfc3339};
use kv::KVStore;
use sql::{SQLStore, Value};
use search::SearchEngine;
use blob::BlobStore;

/// PMS service — holds all storage backends and provides business logic.
pub struct PmsService {
    pub(crate) sql: Box<dyn SQLStore>,
    pub(crate) kv: Box<dyn KVStore>,
    pub(crate) search: Box<dyn SearchEngine>,
    pub(crate) blob: Box<dyn BlobStore>,
}

impl PmsService {
    pub fn new(
        sql: Box<dyn SQLStore>,
        kv: Box<dyn KVStore>,
        search: Box<dyn SearchEngine>,
        blob: Box<dyn BlobStore>,
    ) -> Result<Self, ServiceError> {
        schema::init_schema(sql.as_ref())?;
        Ok(Self { sql, kv, search, blob })
    }

    // ── Generic CRUD helpers ──

    /// Insert a record as JSON into a table with indexed columns.
    pub(crate) fn insert_record<T: Serialize>(
        &self,
        table: &str,
        id: &str,
        record: &T,
        indexes: &[(&str, Value)],
    ) -> Result<(), ServiceError> {
        let json = serde_json::to_string(record)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let mut cols = vec!["id", "data"];
        let mut placeholders = vec!["?1".to_string(), "?2".to_string()];
        let mut params = vec![Value::Text(id.to_string()), Value::Text(json)];

        for (i, (col, val)) in indexes.iter().enumerate() {
            let idx = i + 3;
            cols.push(col);
            placeholders.push(format!("?{}", idx));
            params.push(val.clone());
        }

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            cols.join(", "),
            placeholders.join(", "),
        );

        self.sql.exec(&sql, &params).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE constraint") {
                ServiceError::Conflict(msg)
            } else {
                ServiceError::Storage(msg)
            }
        })?;

        Ok(())
    }

    /// Get a record by id, deserializing the JSON `data` column.
    pub(crate) fn get_record<T: DeserializeOwned>(
        &self,
        table: &str,
        id: &str,
    ) -> Result<T, ServiceError> {
        let sql = format!("SELECT data FROM {} WHERE id = ?1", table);
        let rows = self.sql
            .query(&sql, &[Value::Text(id.to_string())])
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        let row = rows.first()
            .ok_or_else(|| ServiceError::NotFound(format!("{}/{}", table, id)))?;
        let data = row.get_str("data")
            .ok_or_else(|| ServiceError::Internal("missing data column".into()))?;
        serde_json::from_str(data).map_err(|e| ServiceError::Internal(e.to_string()))
    }

    /// Update a record's JSON data and indexed columns.
    pub(crate) fn update_record<T: Serialize>(
        &self,
        table: &str,
        id: &str,
        record: &T,
        indexes: &[(&str, Value)],
    ) -> Result<(), ServiceError> {
        let json = serde_json::to_string(record)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let mut sets = vec!["data = ?1".to_string()];
        let mut params: Vec<Value> = vec![Value::Text(json)];

        for (i, (col, val)) in indexes.iter().enumerate() {
            let idx = i + 2;
            sets.push(format!("{} = ?{}", col, idx));
            params.push(val.clone());
        }

        let id_idx = params.len() + 1;
        params.push(Value::Text(id.to_string()));

        let sql = format!(
            "UPDATE {} SET {} WHERE id = ?{}",
            table,
            sets.join(", "),
            id_idx,
        );

        let affected = self.sql
            .exec(&sql, &params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(ServiceError::NotFound(format!("{}/{}", table, id)));
        }

        Ok(())
    }

    /// Delete a record by id.
    pub(crate) fn delete_record(&self, table: &str, id: &str) -> Result<(), ServiceError> {
        let sql = format!("DELETE FROM {} WHERE id = ?1", table);
        let affected = self.sql
            .exec(&sql, &[Value::Text(id.to_string())])
            .map_err(|e| ServiceError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(ServiceError::NotFound(format!("{}/{}", table, id)));
        }
        Ok(())
    }

    /// List records with optional filters, pagination, and total count.
    pub(crate) fn list_records<T: DeserializeOwned + Serialize>(
        &self,
        table: &str,
        filters: &[(&str, Value)],
        limit: usize,
        offset: usize,
        count: bool,
    ) -> Result<ListResult<T>, ServiceError> {
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();

        for (i, (col, val)) in filters.iter().enumerate() {
            let idx = i + 1;
            where_clauses.push(format!("{} = ?{}", col, idx));
            params.push(val.clone());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        let total = if count {
            let count_sql = format!("SELECT COUNT(*) as cnt FROM {}{}", table, where_sql);
            let rows = self.sql
                .query(&count_sql, &params)
                .map_err(|e| ServiceError::Storage(e.to_string()))?;
            rows.first().and_then(|r| r.get_i64("cnt")).unwrap_or(0) as usize
        } else {
            0
        };

        let limit_idx = params.len() + 1;
        let offset_idx = params.len() + 2;
        params.push(Value::Integer(limit as i64));
        params.push(Value::Integer(offset as i64));

        let sql = format!(
            "SELECT data FROM {}{} ORDER BY create_at DESC LIMIT ?{} OFFSET ?{}",
            table, where_sql, limit_idx, offset_idx,
        );

        let rows = self.sql
            .query(&sql, &params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let mut items = Vec::new();
        for row in &rows {
            let data = row.get_str("data")
                .ok_or_else(|| ServiceError::Internal("missing data column".into()))?;
            let item: T = serde_json::from_str(data)
                .map_err(|e| ServiceError::Internal(e.to_string()))?;
            items.push(item);
        }

        Ok(ListResult { items, total })
    }

    /// Count records with optional filters.
    pub(crate) fn count_records(
        &self,
        table: &str,
        filters: &[(&str, Value)],
    ) -> Result<i64, ServiceError> {
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();

        for (i, (col, val)) in filters.iter().enumerate() {
            let idx = i + 1;
            where_clauses.push(format!("{} = ?{}", col, idx));
            params.push(val.clone());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) as cnt FROM {}{}", table, where_sql);
        let rows = self.sql
            .query(&sql, &params)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        Ok(rows.first().and_then(|r| r.get_i64("cnt")).unwrap_or(0))
    }

    /// Apply a JSON merge-patch to a record.
    pub(crate) fn apply_patch<T: Serialize + DeserializeOwned>(
        current: &T,
        patch: serde_json::Value,
    ) -> Result<T, ServiceError> {
        let mut json = serde_json::to_value(current)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        let now = now_rfc3339();

        // Protect immutable fields
        let mut patch_filtered = patch;
        if let Some(obj) = patch_filtered.as_object_mut() {
            obj.remove("id");
            obj.remove("createAt");
            obj.insert("updateAt".into(), serde_json::json!(now));
        }

        merge_patch(&mut json, &patch_filtered);
        serde_json::from_value(json).map_err(|e| ServiceError::Internal(e.to_string()))
    }
}
