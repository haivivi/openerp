//! Context management commands.

use std::path::PathBuf;

use anyhow::Result;

use crate::config::{ClientConfig, Context};

/// Create a new context — generate server config + register in client config.
pub fn create(
    name: &str,
    config_dir: &str,
    data_dir: &str,
    password: &str,
    client_config_path: &std::path::Path,
) -> Result<()> {
    // Hash the root password with argon2id.
    use argon2::Argon2;
    use password_hash::rand_core::OsRng;
    use password_hash::{PasswordHasher, SaltString};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("failed to hash password: {}", e))?
        .to_string();

    // Generate a random JWT secret.
    let jwt_secret: String = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..32).map(|_| format!("{:02x}", rng.gen::<u8>())).collect()
    };

    // Build server config TOML.
    let server_config = format!(
        r#"[root]
password_hash = "{password_hash}"

[storage]
data_dir = "{data_dir}"

[jwt]
secret = "{jwt_secret}"
expire_secs = 86400
"#
    );

    // Write server config file.
    let config_path = PathBuf::from(config_dir).join(format!("{}.toml", name));
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(&config_path, &server_config)?;

    // Create data directory.
    std::fs::create_dir_all(data_dir)?;

    // Update client config.
    let mut client_config = ClientConfig::load(client_config_path)?;
    client_config.upsert_context(Context {
        name: name.to_string(),
        config_path: config_path.to_string_lossy().to_string(),
        server: String::new(),
        token: String::new(),
    });
    if client_config.current_context.is_empty() {
        client_config.current_context = name.to_string();
    }
    client_config.save(client_config_path)?;

    println!("Context \"{}\" created.", name);
    println!("  Config: {}", config_path.display());
    println!("  Data:   {}", data_dir);

    Ok(())
}

/// List all contexts.
pub fn list(client_config_path: &std::path::Path) -> Result<()> {
    let config = ClientConfig::load(client_config_path)?;

    if config.contexts.is_empty() {
        println!("No contexts configured.");
        println!("Run: openerp context create <name>");
        return Ok(());
    }

    println!("{:2} {:20} {:40} {:12}", "", "NAME", "SERVER", "CONFIG");
    for ctx in &config.contexts {
        let marker = if ctx.name == config.current_context {
            "*"
        } else {
            " "
        };
        let server = if ctx.server.is_empty() { "-" } else { &ctx.server };
        let config_path = if ctx.config_path.is_empty() {
            "-"
        } else {
            &ctx.config_path
        };
        println!("{:2} {:20} {:40} {:12}", marker, ctx.name, server, config_path);
    }

    Ok(())
}

/// Switch current context.
pub fn use_context(name: &str, client_config_path: &std::path::Path) -> Result<()> {
    let mut config = ClientConfig::load(client_config_path)?;

    if !config.contexts.iter().any(|c| c.name == name) {
        anyhow::bail!("Context \"{}\" not found. Run `openerp context list` to see available contexts.", name);
    }

    config.current_context = name.to_string();
    config.save(client_config_path)?;
    println!("Switched to context \"{}\".", name);
    Ok(())
}

/// Set properties on a context.
pub fn set(
    name: &str,
    server: Option<&str>,
    client_config_path: &std::path::Path,
) -> Result<()> {
    let mut config = ClientConfig::load(client_config_path)?;

    let ctx = config
        .get_mut(name)
        .ok_or_else(|| anyhow::anyhow!("Context \"{}\" not found.", name))?;

    if let Some(s) = server {
        ctx.server = s.to_string();
    }

    config.save(client_config_path)?;
    println!("Context \"{}\" updated.", name);
    Ok(())
}

/// Delete a context (doesn't delete server config file).
pub fn delete(name: &str, client_config_path: &std::path::Path) -> Result<()> {
    let mut config = ClientConfig::load(client_config_path)?;

    if !config.remove_context(name) {
        anyhow::bail!("Context \"{}\" not found.", name);
    }

    config.save(client_config_path)?;
    println!("Context \"{}\" deleted.", name);
    Ok(())
}
