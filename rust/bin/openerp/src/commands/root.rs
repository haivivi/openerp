//! Root password management commands.

use anyhow::Result;

use crate::config::ClientConfig;

/// Change root password for the current (or specified) context.
pub fn chpwd(
    context_name: Option<&str>,
    old_password: &str,
    new_password: &str,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;

    let target = context_name.unwrap_or(&config.current_context);
    if target.is_empty() {
        anyhow::bail!("No context specified and no current context set.");
    }

    let ctx = config
        .contexts
        .iter()
        .find(|c| c.name == target)
        .ok_or_else(|| anyhow::anyhow!("Context \"{}\" not found.", target))?;

    if ctx.config_path.is_empty() {
        anyhow::bail!(
            "Context \"{}\" has no local config_path set. Cannot change root password for remote servers.",
            target
        );
    }

    // Load and verify the server config.
    let config_path = std::path::Path::new(&ctx.config_path);
    let server_toml = std::fs::read_to_string(config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", config_path.display(), e))?;

    let mut server_config: toml::Value = toml::from_str(&server_toml)?;

    // Verify old password.
    let current_hash = server_config
        .get("root")
        .and_then(|r| r.get("password_hash"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No root.password_hash in server config"))?;

    // Verify old password matches.
    {
        use argon2::Argon2;
        use password_hash::{PasswordHash, PasswordVerifier};
        let parsed = PasswordHash::new(current_hash)
            .map_err(|e| anyhow::anyhow!("Invalid password hash in config: {}", e))?;
        if Argon2::default()
            .verify_password(old_password.as_bytes(), &parsed)
            .is_err()
        {
            anyhow::bail!("Current root password is incorrect.");
        }
    }

    // Hash new password.
    let new_hash = {
        use argon2::Argon2;
        use password_hash::rand_core::OsRng;
        use password_hash::{PasswordHasher, SaltString};
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash new password: {}", e))?
            .to_string()
    };

    // Update the TOML value.
    if let Some(root) = server_config.get_mut("root") {
        if let Some(table) = root.as_table_mut() {
            table.insert(
                "password_hash".to_string(),
                toml::Value::String(new_hash),
            );
        }
    }

    // Write back.
    let updated_toml = toml::to_string_pretty(&server_config)?;
    std::fs::write(config_path, updated_toml)?;

    println!("Root password updated for context \"{}\".", target);
    println!("NOTE: Restart openerpd for the change to take effect.");
    Ok(())
}
