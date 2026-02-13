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
use openerp_core::Module;
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

    // Initialize embedded stores (shared by all modules).
    let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
        openerp_kv::RedbStore::open(&core_config.resolve_db_path())
            .map_err(|e| anyhow::anyhow!("failed to open KV store: {}", e))?,
    );
    let sql: Arc<dyn openerp_sql::SQLStore> = Arc::new(
        openerp_sql::SqliteStore::open(&core_config.resolve_sqlite_path())
            .map_err(|e| anyhow::anyhow!("failed to open SQL store: {}", e))?,
    );
    let search: Arc<dyn openerp_search::SearchEngine> = Arc::new(
        openerp_search::TantivyEngine::open(&data_dir.join("search"))
            .map_err(|e| anyhow::anyhow!("failed to open search engine: {}", e))?,
    );
    let blob: Arc<dyn openerp_blob::BlobStore> = Arc::new(
        openerp_blob::FileStore::open(&data_dir.join("blob"))
            .map_err(|e| anyhow::anyhow!("failed to open blob store: {}", e))?,
    );
    let tsdb: Arc<dyn openerp_tsdb::TsDb> = Arc::new(
        openerp_tsdb::WalEngine::open(&data_dir.join("tsdb"))
            .map_err(|e| anyhow::anyhow!("failed to open TSDB: {}", e))?,
    );

    // Bootstrap: ensure auth:root role exists.
    bootstrap::ensure_root_role(&kv)?;

    // ── Initialize old modules (will be replaced by DSL versions) ──

    let auth_config = auth::service::AuthConfig {
        jwt_secret: server_config.jwt.secret.clone(),
        ..Default::default()
    };
    let auth_module = auth::AuthModule::new(
        Arc::clone(&sql),
        Arc::clone(&kv),
        auth_config,
    )?;
    info!("Auth module initialized (legacy)");

    let pms_module = pms::PmsModule::new(
        Arc::clone(&sql),
        Arc::clone(&kv),
        Arc::clone(&search),
        Arc::clone(&blob),
    )?;
    info!("PMS module initialized");

    let task_module = task::TaskModule::new(
        Arc::clone(&sql),
        Arc::clone(&kv),
        Arc::clone(&tsdb),
    )?;
    info!("Task module initialized");

    // Legacy module routes (old API, kept for compatibility).
    let module_routes = vec![
        (auth_module.name(), auth_module.routes()),
        (pms_module.name(), pms_module.routes()),
        (task_module.name(), task_module.routes()),
    ];

    // ── DSL-based modules (new admin API) ──

    let authenticator: Arc<dyn openerp_core::Authenticator> =
        Arc::new(openerp_core::AllowAll); // TODO: use AuthChecker once JWT middleware injects roles

    let admin_routes: Vec<(&str, axum::Router)> = vec![
        ("auth", auth_v2::admin_router(Arc::clone(&kv), authenticator.clone())),
    ];
    info!("Auth v2 admin router mounted at /admin/auth/");

    // ── Schema (auto-generated from DSL + UI overrides) ──

    let mut schema_json = openerp_store::build_schema(
        "OpenERP",
        vec![auth_v2::schema_def()],
    );
    openerp_store::apply_overrides(&mut schema_json, &auth_v2::ui_overrides());

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
    let app = routes::build_router(app_state, module_routes, admin_routes, schema_json);

    // Start server.
    let listener = tokio::net::TcpListener::bind(&cli.listen).await?;
    info!("OpenERP server listening on {}", cli.listen);
    axum::serve(listener, app).await?;

    Ok(())
}
