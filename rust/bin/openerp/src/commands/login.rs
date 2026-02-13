//! Login / logout commands.

use anyhow::Result;

use crate::config::ClientConfig;

/// Login to the current context's server.
pub fn login(
    username: &str,
    password: &str,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let mut config = ClientConfig::load(client_config_path)?;

    let ctx = config
        .current()
        .ok_or_else(|| anyhow::anyhow!("No current context. Run `openerp use context <name>`."))?
        .clone();

    if ctx.server.is_empty() {
        anyhow::bail!(
            "No server URL set for context \"{}\". Run `openerp context set {} --server <url>`.",
            ctx.name,
            ctx.name
        );
    }

    // Send login request.
    let url = format!("{}/auth/login", ctx.server.trim_end_matches('/'));
    let body = serde_json::json!({
        "username": username,
        "password": password,
    });

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| anyhow::anyhow!("failed to connect to server: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Login failed ({}): {}", status, text);
    }

    let data: serde_json::Value = resp.json()?;
    let token = data["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No access_token in response"))?;

    // Save token to context.
    let ctx_mut = config
        .get_mut(&ctx.name)
        .ok_or_else(|| anyhow::anyhow!("Context disappeared"))?;
    ctx_mut.token = token.to_string();
    config.save(client_config_path)?;

    println!("Logged in as {}.", username);
    println!("Token saved to context \"{}\".", ctx.name);
    Ok(())
}

/// Logout — clear token from current context.
pub fn logout(client_config_path: &std::path::Path) -> Result<()> {
    let mut config = ClientConfig::load(client_config_path)?;

    let current_name = config.current_context.clone();
    if current_name.is_empty() {
        anyhow::bail!("No current context.");
    }

    let ctx = config
        .get_mut(&current_name)
        .ok_or_else(|| anyhow::anyhow!("Current context not found."))?;

    ctx.token = String::new();
    config.save(client_config_path)?;
    println!("Logged out from context \"{}\".", current_name);
    Ok(())
}
