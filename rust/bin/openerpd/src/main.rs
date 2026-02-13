//! `openerpd` — the OpenERP server binary.
//!
//! Usage:
//!   openerpd -c <context-name-or-path> [--listen <addr>]
//!
//! The context name resolves to `/etc/openerp/<name>.toml`.
//! If a path with `/` or `.` is given, it's used directly.

mod auth_middleware;
mod bootstrap;
mod config;
mod login;
mod routes;

use std::sync::Arc;

use clap::Parser;
use jsonwebtoken::{DecodingKey, Validation};
use tracing::info;

use auth_middleware::JwtState;
use config::ServerConfig;
use routes::AppState;

/// OpenERP server.
#[derive(Parser, Debug)]
#[command(name = "openerpd", about = "OpenERP server")]
struct Cli {
    /// Context name or path to config file.
    #[arg(short = 'c', long = "config", required = true)]
    config: String,

    /// Listen address (overrides default 0.0.0.0:8080).
    #[arg(long = "listen", default_value = "0.0.0.0:8080")]
    listen: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    // Load server configuration.
    let config_path = ServerConfig::resolve_path(&cli.config);
    info!("Loading configuration from {}", config_path.display());
    let server_config = ServerConfig::load(&config_path)?;

    // Verify configuration is valid.
    bootstrap::verify_config(&server_config)?;

    // Initialize storage.
    let data_dir = std::path::PathBuf::from(&server_config.storage.data_dir);
    std::fs::create_dir_all(&data_dir)?;

    let core_config = openerp_core::ServiceConfig {
        data_dir: Some(data_dir.clone()),
        listen: cli.listen.clone(),
        ..Default::default()
    };

    // Initialize embedded stores.
    let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
        openerp_kv::RedbStore::open(&core_config.resolve_db_path())
            .map_err(|e| anyhow::anyhow!("failed to open KV store: {}", e))?,
    );
    let sql: Arc<dyn openerp_sql::SQLStore> = Arc::new(
        openerp_sql::SqliteStore::open(&core_config.resolve_sqlite_path())
            .map_err(|e| anyhow::anyhow!("failed to open SQL store: {}", e))?,
    );

    // Bootstrap: ensure auth:root role exists.
    bootstrap::ensure_root_role(&kv)?;

    // Build JWT state for middleware.
    let jwt_state = Arc::new(JwtState {
        decoding_key: DecodingKey::from_secret(server_config.jwt.secret.as_bytes()),
        validation: Validation::default(),
    });

    let server_config = Arc::new(server_config);

    // Build application state.
    let app_state = AppState {
        jwt_state,
        server_config,
        kv,
        sql,
    };

    // Build router.
    let app = routes::build_router(app_state);

    // Start server.
    let listener = tokio::net::TcpListener::bind(&cli.listen).await?;
    info!("OpenERP server listening on {}", cli.listen);
    axum::serve(listener, app).await?;

    Ok(())
}
