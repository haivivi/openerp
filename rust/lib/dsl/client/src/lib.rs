//! OpenERP DSL Client — typed HTTP client for admin CRUD APIs.
//!
//! Generic `ResourceClient<T>` provides CRUD operations for any model.
//! The codegen creates type aliases: `pub type UsersClient = ResourceClient<User>;`
//!
//! Golden test: hand-written. Production: auto-generated per #[model].

use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// HTTP client error.
#[derive(Debug)]
pub enum ClientError {
    Network(String),
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    Server(u16, String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Network(msg) => write!(f, "network error: {}", msg),
            ClientError::NotFound(msg) => write!(f, "not found: {}", msg),
            ClientError::Conflict(msg) => write!(f, "conflict: {}", msg),
            ClientError::BadRequest(msg) => write!(f, "bad request: {}", msg),
            ClientError::Server(code, msg) => write!(f, "server error {}: {}", code, msg),
        }
    }
}

impl std::error::Error for ClientError {}

/// List response from the admin API.
#[derive(Debug, serde::Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
}

/// Generic CRUD client for a single resource type.
///
/// Usage (generated per model):
/// ```ignore
/// let users: ResourceClient<User> = ResourceClient::new(&http, base_url, "users");
/// let all = users.list().await?;
/// let alice = users.get("alice").await?;
/// let created = users.create(&new_user).await?;
/// ```
pub struct ResourceClient<T> {
    http: reqwest::Client,
    /// Full URL prefix: e.g. "http://127.0.0.1:3000/admin/twitter/users"
    url: String,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned> ResourceClient<T> {
    pub fn new(http: &reqwest::Client, base_url: &str, resource_path: &str) -> Self {
        Self {
            http: http.clone(),
            url: format!("{}/{}", base_url.trim_end_matches('/'), resource_path),
            _phantom: PhantomData,
        }
    }

    /// List all records.
    pub async fn list(&self) -> Result<Vec<T>, ClientError> {
        let resp = self.http.get(&self.url).send().await
            .map_err(|e| ClientError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            return Err(self.parse_error(status, resp).await);
        }
        let list: ListResponse<T> = resp.json().await
            .map_err(|e| ClientError::Network(e.to_string()))?;
        Ok(list.items)
    }

    /// Get a single record by ID.
    pub async fn get(&self, id: &str) -> Result<T, ClientError> {
        let url = format!("{}/{}", self.url, id);
        let resp = self.http.get(&url).send().await
            .map_err(|e| ClientError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status == 404 {
            return Err(ClientError::NotFound(id.to_string()));
        }
        if status != 200 {
            return Err(self.parse_error(status, resp).await);
        }
        resp.json().await.map_err(|e| ClientError::Network(e.to_string()))
    }

    /// Create a new record.
    pub async fn create(&self, record: &T) -> Result<T, ClientError> {
        let resp = self.http.post(&self.url).json(record).send().await
            .map_err(|e| ClientError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 && status != 201 {
            return Err(self.parse_error(status, resp).await);
        }
        resp.json().await.map_err(|e| ClientError::Network(e.to_string()))
    }

    /// Update a record by ID (full replace).
    pub async fn update(&self, id: &str, record: &T) -> Result<T, ClientError> {
        let url = format!("{}/{}", self.url, id);
        let resp = self.http.put(&url).json(record).send().await
            .map_err(|e| ClientError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 {
            return Err(self.parse_error(status, resp).await);
        }
        resp.json().await.map_err(|e| ClientError::Network(e.to_string()))
    }

    /// Delete a record by ID.
    pub async fn delete(&self, id: &str) -> Result<(), ClientError> {
        let url = format!("{}/{}", self.url, id);
        let resp = self.http.delete(&url).send().await
            .map_err(|e| ClientError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        if status != 200 && status != 204 {
            return Err(self.parse_error(status, resp).await);
        }
        Ok(())
    }

    async fn parse_error(&self, status: u16, resp: reqwest::Response) -> ClientError {
        let body = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["error"].as_str().map(|s| s.to_string()))
            .unwrap_or(body);
        match status {
            404 => ClientError::NotFound(msg),
            409 => ClientError::Conflict(msg),
            400 => ClientError::BadRequest(msg),
            _ => ClientError::Server(status, msg),
        }
    }
}
