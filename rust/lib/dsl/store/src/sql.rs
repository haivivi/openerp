//! SqlStore trait + SqlOps CRUD operations.
//!
//! Models impl `SqlStore` to declare PK, UNIQUE, INDEX.
//! `SqlOps<T>` provides CRUD + filtered queries using SQLStore backend.
//!
//! Data is stored as JSON blob in a `data` column, with indexed fields
//! extracted into dedicated columns for efficient queries.

use oe_core::ServiceError;
use oe_types::Field;
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
    sql: Arc<dyn oe_sql::SQLStore>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: SqlStore> SqlOps<T> {
    pub fn new(sql: Arc<dyn oe_sql::SQLStore>) -> Self {
        Self {
            sql,
            _phantom: std::marker::PhantomData,
        }
    }

    fn sql_err(e: oe_sql::SQLError) -> ServiceError {
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
        let params: Vec<oe_sql::Value> = pk
            .iter()
            .map(|v| oe_sql::Value::Text(v.to_string()))
            .collect();

        let rows = self.sql.query(&sql, &params).map_err(Self::sql_err)?;
        if let Some(row) = rows.first() {
            if let Some(oe_sql::Value::Blob(data)) = row.get("data") {
                let record: T = serde_json::from_slice(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                return Ok(Some(record));
            }
            if let Some(oe_sql::Value::Text(data)) = row.get("data") {
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
        let mut records = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(oe_sql::Value::Blob(data)) = row.get("data") {
                let record: T = serde_json::from_slice(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            } else if let Some(oe_sql::Value::Text(data)) = row.get("data") {
                let record: T = serde_json::from_str(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            }
        }
        Ok(records)
    }

    /// Insert a new record. Calls before_create.
    pub fn save_new(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_create();

        let data = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        let indexed = T::indexed_fields();
        let json_val: serde_json::Value = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        // Build INSERT.
        let mut col_names = Vec::new();
        let mut placeholders = Vec::new();
        let mut params: Vec<oe_sql::Value> = Vec::new();

        for (i, f) in indexed.iter().enumerate() {
            col_names.push(format!("\"{}\"", f.name));
            placeholders.push(format!("?{}", i + 1));
            let val = json_val
                .get(f.name)
                .or_else(|| json_val.get(&to_camel_case(f.name)))
                .map(|v| match v {
                    serde_json::Value::String(s) => oe_sql::Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        oe_sql::Value::Integer(n.as_i64().unwrap_or(0))
                    }
                    serde_json::Value::Bool(b) => oe_sql::Value::Integer(*b as i64),
                    other => oe_sql::Value::Text(other.to_string()),
                })
                .unwrap_or(oe_sql::Value::Null);
            params.push(val);
        }

        col_names.push("data".to_string());
        placeholders.push(format!("?{}", indexed.len() + 1));
        params.push(oe_sql::Value::Blob(data));

        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            T::table_name(),
            col_names.join(", "),
            placeholders.join(", ")
        );

        self.sql.exec(&sql, &params).map_err(Self::sql_err)?;
        Ok(record)
    }

    /// Update an existing record. Calls before_update.
    pub fn save(&self, mut record: T) -> Result<T, ServiceError> {
        record.before_update();

        let data = serde_json::to_vec(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        let indexed = T::indexed_fields();
        let json_val: serde_json::Value = serde_json::to_value(&record)
            .map_err(|e| ServiceError::Internal(format!("serialize: {}", e)))?;

        let pk_fields = T::PK;
        let pk_values = record.pk_values();

        // Build UPDATE.
        let mut set_clauses = Vec::new();
        let mut params: Vec<oe_sql::Value> = Vec::new();
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
                    serde_json::Value::String(s) => oe_sql::Value::Text(s.clone()),
                    serde_json::Value::Number(n) => {
                        oe_sql::Value::Integer(n.as_i64().unwrap_or(0))
                    }
                    serde_json::Value::Bool(b) => oe_sql::Value::Integer(*b as i64),
                    other => oe_sql::Value::Text(other.to_string()),
                })
                .unwrap_or(oe_sql::Value::Null);
            params.push(val);
            idx += 1;
        }

        set_clauses.push(format!("data = ?{}", idx));
        params.push(oe_sql::Value::Blob(data));
        idx += 1;

        let where_clause: Vec<String> = pk_fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                params.push(oe_sql::Value::Text(pk_values[i].clone()));
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
        Ok(record)
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
        let params: Vec<oe_sql::Value> = pk
            .iter()
            .map(|v| oe_sql::Value::Text(v.to_string()))
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
            .query(&sql, &[oe_sql::Value::Text(value.to_string())])
            .map_err(Self::sql_err)?;

        let mut records = Vec::new();
        for row in &rows {
            if let Some(oe_sql::Value::Blob(data)) = row.get("data") {
                let record: T = serde_json::from_slice(data)
                    .map_err(|e| ServiceError::Internal(format!("deserialize: {}", e)))?;
                records.push(record);
            } else if let Some(oe_sql::Value::Text(data)) = row.get("data") {
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
        let sql: Arc<dyn oe_sql::SQLStore> =
            Arc::new(oe_sql::SqliteStore::open(&dir.path().join("test.db")).unwrap());
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
        let sql: Arc<dyn oe_sql::SQLStore> =
            Arc::new(oe_sql::SqliteStore::open(&dir.path().join("test2.db")).unwrap());
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
}
