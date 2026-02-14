//! `openerpd` — the OpenERP server binary.
//!
//! Usage:
//!   openerpd -c <context-name-or-path> [--listen <addr>]

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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    let config_path = ServerConfig::resolve_path(&cli.config);
    info!("Loading configuration from {}", config_path.display());
    let server_config = ServerConfig::load(&config_path)?;

    bootstrap::verify_config(&server_config)?;

    // Initialize storage.
    let data_dir = std::path::PathBuf::from(&server_config.storage.data_dir);
    std::fs::create_dir_all(&data_dir)?;

    let core_config = openerp_core::ServiceConfig {
        data_dir: Some(data_dir.clone()),
        listen: cli.listen.clone(),
        ..Default::default()
    };

    let kv: Arc<dyn openerp_kv::KVStore> = Arc::new(
        openerp_kv::RedbStore::open(&core_config.resolve_db_path())
            .map_err(|e| anyhow::anyhow!("failed to open KV store: {}", e))?,
    );

    // Bootstrap: ensure auth:root role exists.
    bootstrap::ensure_root_role(&kv)?;

    // ── DSL modules ──

    let authenticator: Arc<dyn openerp_core::Authenticator> =
        Arc::new(openerp_core::AllowAll); // TODO: use AuthChecker

    let admin_routes: Vec<(&str, axum::Router)> = vec![
        ("auth", auth::admin_router(Arc::clone(&kv), authenticator.clone())),
        ("pms", pms::admin_router(Arc::clone(&kv), authenticator.clone())),
        ("task", task::admin_router(Arc::clone(&kv), authenticator.clone())),
    ];
    info!("Admin routers: /admin/auth/, /admin/pms/, /admin/task/");

    // ── Facet routes (multi-consumer APIs) ──

    let mut facet_routes: Vec<openerp_store::FacetDef> = Vec::new();
    facet_routes.extend(auth::facet_routers(Arc::clone(&kv)));
    facet_routes.extend(pms::facet_routers(Arc::clone(&kv)));
    facet_routes.extend(task::facet_routers(Arc::clone(&kv)));
    if !facet_routes.is_empty() {
        let names: Vec<String> = facet_routes.iter()
            .map(|f| format!("/{}/{}", f.name, f.module))
            .collect();
        info!("Facet routers: {}", names.join(", "));
    }

    // ── Schema (auto-generated from DSL + UI overrides) ──

    let mut schema_json = openerp_store::build_schema(
        "OpenERP",
        vec![
            auth::schema_def(),
            pms::schema_def(),
            task::schema_def(),
        ],
    );
    openerp_store::apply_overrides(&mut schema_json, &auth::ui_overrides());

    // Build JWT state for middleware.
    let jwt_state = Arc::new(JwtState {
        decoding_key: DecodingKey::from_secret(server_config.jwt.secret.as_bytes()),
        validation: Validation::default(),
    });

    let server_config = Arc::new(server_config);

    let app_state = AppState {
        jwt_state,
        server_config,
        kv,
    };

    let app = routes::build_router(app_state, admin_routes, facet_routes, schema_json);

    let listener = tokio::net::TcpListener::bind(&cli.listen).await?;
    info!("OpenERP server listening on {}", cli.listen);
    axum::serve(listener, app).await?;

    Ok(())
}
