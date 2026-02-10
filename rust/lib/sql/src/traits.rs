use crate::error::SQLError;

/// A dynamically-typed SQL parameter value.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// A row returned from a SQL query — column name to value.
#[derive(Debug, Clone)]
pub struct Row {
    pub columns: Vec<(String, Value)>,
}

impl Row {
    /// Get a column value by name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.columns.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// Get a text column value by name.
    pub fn get_str(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(Value::Text(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get an integer column value by name.
    pub fn get_i64(&self, name: &str) -> Option<i64> {
        match self.get(name) {
            Some(Value::Integer(i)) => Some(*i),
            _ => None,
        }
    }

    /// Get a real column value by name.
    pub fn get_f64(&self, name: &str) -> Option<f64> {
        match self.get(name) {
            Some(Value::Real(f)) => Some(*f),
            _ => None,
        }
    }
}

/// SQLStore provides a SQL execution interface backed by an embedded database.
pub trait SQLStore: Send + Sync {
    /// Execute a query and return rows.
    fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Row>, SQLError>;

    /// Execute a statement (INSERT/UPDATE/DELETE) and return affected row count.
    fn exec(&self, sql: &str, params: &[Value]) -> Result<u64, SQLError>;
}
