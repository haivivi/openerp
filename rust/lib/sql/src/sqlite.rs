use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::SQLError;
use crate::traits::{Row, SQLStore, Value};

/// SqliteStore is a SQLStore implementation backed by rusqlite (bundled SQLite).
pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Open or create a SQLite database at the given path.
    pub fn open(path: &Path) -> Result<Self, SQLError> {
        let conn = Connection::open(path)
            .map_err(|e| SQLError::Connection(e.to_string()))?;

        // Enable WAL mode for better concurrent read performance.
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| SQLError::Connection(e.to_string()))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create an in-memory SQLite database (useful for tests).
    pub fn open_in_memory() -> Result<Self, SQLError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| SQLError::Connection(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

/// Convert our Value enum to rusqlite's ToSql.
fn bind_params(params: &[Value]) -> Vec<Box<dyn rusqlite::types::ToSql + '_>> {
    params
        .iter()
        .map(|v| -> Box<dyn rusqlite::types::ToSql + '_> {
            match v {
                Value::Null => Box::new(rusqlite::types::Null),
                Value::Integer(i) => Box::new(*i),
                Value::Real(f) => Box::new(*f),
                Value::Text(s) => Box::new(s.as_str()),
                Value::Blob(b) => Box::new(b.as_slice()),
            }
        })
        .collect()
}

impl SQLStore for SqliteStore {
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>, SQLError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SQLError::Query(e.to_string()))?;

        let bound = bind_params(params);
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            bound.iter().map(|b| b.as_ref()).collect();

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| SQLError::Query(e.to_string()))?;

        let column_names: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let mut columns = Vec::new();
                for (i, name) in column_names.iter().enumerate() {
                    let val = row_value_at(row, i);
                    columns.push((name.clone(), val));
                }
                Ok(Row { columns })
            })
            .map_err(|e| SQLError::Query(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| SQLError::Query(e.to_string()))?);
        }
        Ok(result)
    }

    fn exec(&self, sql: &str, params: &[Value]) -> Result<u64, SQLError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SQLError::Execution(e.to_string()))?;

        let bound = bind_params(params);
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            bound.iter().map(|b| b.as_ref()).collect();

        let affected = conn
            .execute(sql, param_refs.as_slice())
            .map_err(|e| SQLError::Execution(e.to_string()))?;

        Ok(affected as u64)
    }
}

/// Extract a Value from a rusqlite row at a given column index.
fn row_value_at(row: &rusqlite::Row, idx: usize) -> Value {
    // Try integer first, then real, then text, then blob, then null.
    if let Ok(i) = row.get::<_, i64>(idx) {
        return Value::Integer(i);
    }
    if let Ok(f) = row.get::<_, f64>(idx) {
        return Value::Real(f);
    }
    if let Ok(s) = row.get::<_, String>(idx) {
        return Value::Text(s);
    }
    if let Ok(b) = row.get::<_, Vec<u8>>(idx) {
        return Value::Blob(b);
    }
    Value::Null
}
