pub mod api;
pub mod schema;
pub mod user;
pub mod group;
pub mod provider;
pub mod role;
pub mod policy;
pub mod session;
pub mod expansion;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

use openerp_kv::KVStore;
use openerp_sql::{SQLStore, Value};

/// Auth service error type.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("storage: {0}")]
    Storage(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl From<AuthError> for openerp_core::ServiceError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::NotFound(m) => openerp_core::ServiceError::NotFound(m),
            AuthError::Conflict(m) => openerp_core::ServiceError::Conflict(m),
            AuthError::Validation(m) => openerp_core::ServiceError::Validation(m),
            AuthError::Unauthorized(m) | AuthError::Forbidden(m) => {
                openerp_core::ServiceError::ReadOnly(m)
            }
            AuthError::Storage(m) => openerp_core::ServiceError::Storage(m),
            AuthError::Internal(m) => openerp_core::ServiceError::Internal(m),
        }
    }
}

/// Configuration for the auth service.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT signing secret.
    pub jwt_secret: String,
    /// Access token lifetime in seconds (default: 24h).
    pub access_token_ttl: i64,
    /// Refresh token lifetime in seconds (default: 7 days).
    pub refresh_token_ttl: i64,
    /// Group expansion cache TTL in seconds (default: 120).
    pub group_cache_ttl: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "openerp-dev-secret-change-me".to_string(),
            access_token_ttl: 86400,       // 24h
            refresh_token_ttl: 604800,     // 7 days
            group_cache_ttl: 120,          // 2 min
        }
    }
}

/// The Auth service. Holds storage backends and configuration.
pub struct AuthService {
    pub(crate) sql: Arc<dyn SQLStore>,
    pub(crate) kv: Arc<dyn KVStore>,
    pub(crate) config: AuthConfig,
    pub(crate) group_cache: expansion::GroupCache,
}

impl AuthService {
    /// Create a new AuthService, initializing the DB schema.
    pub fn new(
        sql: Arc<dyn SQLStore>,
        kv: Arc<dyn KVStore>,
        config: AuthConfig,
    ) -> Result<Arc<Self>, AuthError> {
        schema::init_schema(sql.as_ref())?;
        let group_cache_ttl = config.group_cache_ttl;
        let svc = Arc::new(Self {
            sql,
            kv,
            config,
            group_cache: expansion::GroupCache::new(group_cache_ttl),
        });
        Ok(svc)
    }

    // ── Generic CRUD helpers (same pattern as PmsService) ──

    /// Insert a record as JSON into a table with indexed columns.
    pub(crate) fn insert_record<T: Serialize>(
        &self,
        table: &str,
        id: &str,
        record: &T,
        indexes: &[(&str, Value)],
    ) -> Result<(), AuthError> {
        let json = serde_json::to_string(record)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

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
                AuthError::Conflict(msg)
            } else {
                AuthError::Storage(msg)
            }
        })?;

        Ok(())
    }

    /// Get a record by id, deserializing the JSON `data` column.
    pub(crate) fn get_record<T: DeserializeOwned>(
        &self,
        table: &str,
        id: &str,
    ) -> Result<T, AuthError> {
        let sql = format!("SELECT data FROM {} WHERE id = ?1", table);
        let rows = self.sql
            .query(&sql, &[Value::Text(id.to_string())])
            .map_err(|e| AuthError::Storage(e.to_string()))?;
        let row = rows
            .first()
            .ok_or_else(|| AuthError::NotFound(format!("{}/{}", table, id)))?;
        let data = row
            .get_str("data")
            .ok_or_else(|| AuthError::Internal("missing data column".into()))?;
        serde_json::from_str(data).map_err(|e| AuthError::Internal(e.to_string()))
    }

    /// Update a record's JSON data and indexed columns.
    pub(crate) fn update_record<T: Serialize>(
        &self,
        table: &str,
        id: &str,
        record: &T,
        indexes: &[(&str, Value)],
    ) -> Result<(), AuthError> {
        let json = serde_json::to_string(record)
            .map_err(|e| AuthError::Internal(e.to_string()))?;

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
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        if affected == 0 {
            return Err(AuthError::NotFound(format!("{}/{}", table, id)));
        }

        Ok(())
    }

    /// Delete a record by id.
    pub(crate) fn delete_record(&self, table: &str, id: &str) -> Result<(), AuthError> {
        let sql = format!("DELETE FROM {} WHERE id = ?1", table);
        let affected = self.sql
            .exec(&sql, &[Value::Text(id.to_string())])
            .map_err(|e| AuthError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(AuthError::NotFound(format!("{}/{}", table, id)));
        }
        Ok(())
    }

    /// List records with optional filters and pagination.
    pub(crate) fn list_records<T: DeserializeOwned + Serialize>(
        &self,
        table: &str,
        filters: &[(&str, Value)],
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<T>, usize), AuthError> {
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

        // Count
        let count_sql = format!("SELECT COUNT(*) as cnt FROM {}{}", table, where_sql);
        let count_rows = self.sql
            .query(&count_sql, &params)
            .map_err(|e| AuthError::Storage(e.to_string()))?;
        let total = count_rows
            .first()
            .and_then(|r| r.get_i64("cnt"))
            .unwrap_or(0) as usize;

        // Items
        let limit_idx = params.len() + 1;
        let offset_idx = params.len() + 2;
        params.push(Value::Integer(limit as i64));
        params.push(Value::Integer(offset as i64));

        let sql = format!(
            "SELECT data FROM {}{} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
            table, where_sql, limit_idx, offset_idx,
        );

        let rows = self.sql
            .query(&sql, &params)
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        let mut items = Vec::new();
        for row in &rows {
            let data = row
                .get_str("data")
                .ok_or_else(|| AuthError::Internal("missing data column".into()))?;
            let item: T =
                serde_json::from_str(data).map_err(|e| AuthError::Internal(e.to_string()))?;
            items.push(item);
        }

        Ok((items, total))
    }
}
