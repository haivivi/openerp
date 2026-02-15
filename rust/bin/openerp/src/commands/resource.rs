//! Generic resource CRUD commands.
//!
//! `openerp get users`, `openerp create provider`, etc.
//! Translates resource names to REST API paths.

use anyhow::Result;

use crate::config::ClientConfig;

/// Map a singular/plural resource name to the API path prefix.
fn resource_path(resource: &str) -> Result<(&'static str, &'static str)> {
    // Returns (singular, api_path).
    match resource.to_lowercase().as_str() {
        // Auth
        "user" | "users" => Ok(("user", "/auth/users")),
        "session" | "sessions" => Ok(("session", "/auth/sessions")),
        "role" | "roles" => Ok(("role", "/auth/roles")),
        "group" | "groups" => Ok(("group", "/auth/groups")),
        "policy" | "policies" => Ok(("policy", "/auth/policies")),
        "provider" | "providers" => Ok(("provider", "/auth/providers")),
        // PMS
        "device" | "devices" => Ok(("device", "/pms/devices")),
        "batch" | "batches" => Ok(("batch", "/pms/batches")),
        "license" | "licenses" => Ok(("license", "/pms/licenses")),
        "firmware" | "firmwares" => Ok(("firmware", "/pms/firmwares")),
        "model" | "models" => Ok(("model", "/pms/models")),
        "segment" | "segments" => Ok(("segment", "/pms/segments")),
        "license-import" | "license-imports" | "licenseimport" | "licenseimports"
            => Ok(("license-import", "/pms/license-imports")),
        // Task
        "task" | "tasks" => Ok(("task", "/task/tasks")),
        "task-type" | "task-types" | "tasktype" | "tasktypes"
            => Ok(("task-type", "/task/task-types")),
        _ => Err(anyhow::anyhow!("Unknown resource type: {}", resource)),
    }
}

/// HTTP client helper.
fn build_client(ctx: &crate::config::Context) -> Result<(reqwest::blocking::Client, String)> {
    if ctx.server.is_empty() {
        anyhow::bail!(
            "No server URL set for context \"{}\". Run `openerp context set {} --server <url>`.",
            ctx.name, ctx.name
        );
    }

    let mut headers = reqwest::header::HeaderMap::new();
    if !ctx.token.is_empty() {
        let val = format!("Bearer {}", ctx.token);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&val)?,
        );
    }

    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()?;

    Ok((client, ctx.server.trim_end_matches('/').to_string()))
}

/// GET a resource (list or get by ID).
pub fn get(
    resource: &str,
    id: Option<&str>,
    output_json: bool,
    limit: Option<usize>,
    offset: Option<usize>,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;
    let ctx = config
        .current()
        .ok_or_else(|| anyhow::anyhow!("No current context."))?;

    let (_, api_path) = resource_path(resource)?;
    let (client, base_url) = build_client(ctx)?;

    let url = if let Some(id) = id {
        format!("{}{}/{}", base_url, api_path, id)
    } else {
        let mut u = format!("{}{}", base_url, api_path);
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            u.push('?');
            u.push_str(&params.join("&"));
        }
        u
    };

    let resp = client.get(&url).send()?;
    let status = resp.status();
    let body: serde_json::Value = resp.json()?;

    if !status.is_success() {
        let error = body["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("Error ({}): {}", status, error);
    }

    if output_json {
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        // Simple table output.
        println!("{}", serde_json::to_string_pretty(&body)?);
    }
    Ok(())
}

/// CREATE a resource.
pub fn create(
    resource: &str,
    json_body: &str,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;
    let ctx = config
        .current()
        .ok_or_else(|| anyhow::anyhow!("No current context."))?;

    let (singular, api_path) = resource_path(resource)?;
    let (client, base_url) = build_client(ctx)?;

    let url = format!("{}{}", base_url, api_path);
    let body: serde_json::Value = serde_json::from_str(json_body)
        .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let resp = client.post(&url).json(&body).send()?;
    let status = resp.status();
    let result: serde_json::Value = resp.json()?;

    if !status.is_success() {
        let error = result["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("Error ({}): {}", status, error);
    }

    println!("{} created.", singular);
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// DELETE a resource.
pub fn delete(
    resource: &str,
    id: &str,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;
    let ctx = config
        .current()
        .ok_or_else(|| anyhow::anyhow!("No current context."))?;

    let (singular, api_path) = resource_path(resource)?;
    let (client, base_url) = build_client(ctx)?;

    let url = format!("{}{}/{}", base_url, api_path, id);
    let resp = client.delete(&url).send()?;
    let status = resp.status();

    if !status.is_success() {
        let body: serde_json::Value = resp.json().unwrap_or_default();
        let error = body["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("Error ({}): {}", status, error);
    }

    println!("{} {} deleted.", singular, id);
    Ok(())
}

/// UPDATE a resource (PATCH).
pub fn update(
    resource: &str,
    id: &str,
    json_body: &str,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;
    let ctx = config
        .current()
        .ok_or_else(|| anyhow::anyhow!("No current context."))?;

    let (singular, api_path) = resource_path(resource)?;
    let (client, base_url) = build_client(ctx)?;

    let url = format!("{}{}/{}", base_url, api_path, id);
    let body: serde_json::Value = serde_json::from_str(json_body)
        .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    let resp = client.patch(&url).json(&body).send()?;
    let status = resp.status();
    let result: serde_json::Value = resp.json()?;

    if !status.is_success() {
        let error = result["error"].as_str().unwrap_or("unknown error");
        anyhow::bail!("Error ({}): {}", status, error);
    }

    println!("{} {} updated.", singular, id);
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// STATUS — check server health.
pub fn status(client_config_path: &std::path::Path) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;
    let ctx = config
        .current()
        .ok_or_else(|| anyhow::anyhow!("No current context."))?;

    println!("Context:   {}", ctx.name);
    println!("Server:    {}", if ctx.server.is_empty() { "-" } else { &ctx.server });

    if ctx.server.is_empty() {
        println!("Status:    no server configured");
        return Ok(());
    }

    let (client, base_url) = build_client(ctx)?;
    match client.get(&format!("{}/health", base_url)).send() {
        Ok(resp) if resp.status().is_success() => {
            println!("Status:    connected");
        }
        Ok(resp) => {
            println!("Status:    error ({})", resp.status());
        }
        Err(e) => {
            println!("Status:    disconnected ({})", e);
        }
    }
    Ok(())
}
