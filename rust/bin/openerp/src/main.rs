//! `openerp` — the OpenERP CLI client.
//!
//! Manages contexts, authentication, and resource operations.
//! Think of it as `kubectl` for OpenERP.

mod commands;
mod config;

use clap::{Parser, Subcommand};

/// OpenERP CLI tool.
#[derive(Parser, Debug)]
#[command(name = "openerp", about = "OpenERP CLI client")]
struct Cli {
    /// Path to client config file (default: ~/.openerp/config.toml).
    #[arg(long = "config", global = true)]
    config: Option<String>,

    /// Output format: table or json.
    #[arg(long = "output", short = 'o', global = true, default_value = "table")]
    output: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new context (generates server config + sets root password).
    #[command(name = "context")]
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    /// Switch the current context.
    #[command(name = "use")]
    Use {
        #[command(subcommand)]
        what: UseWhat,
    },

    /// Root account management.
    Root {
        #[command(subcommand)]
        action: RootAction,
    },

    /// Login to the current context's server.
    Login {
        /// Username.
        #[arg(long)]
        user: Option<String>,
        /// Password (not recommended — use interactive prompt).
        #[arg(long)]
        password: Option<String>,
    },

    /// Logout — clear token from current context.
    Logout,

    /// Get resource(s).
    Get {
        /// Resource type (e.g. users, devices, tasks).
        resource: String,
        /// Optional resource ID for single get.
        id: Option<String>,
        /// Limit results.
        #[arg(long)]
        limit: Option<usize>,
        /// Offset for pagination.
        #[arg(long)]
        offset: Option<usize>,
    },

    /// Create a resource.
    Create {
        /// Resource type.
        resource: String,
        /// JSON body.
        #[arg(long = "json")]
        json_body: Option<String>,
        /// Read JSON from file.
        #[arg(short = 'f', long = "file")]
        file: Option<String>,
    },

    /// Update a resource (PATCH).
    Update {
        /// Resource type.
        resource: String,
        /// Resource ID.
        id: String,
        /// JSON body.
        #[arg(long = "json")]
        json_body: String,
    },

    /// Delete a resource.
    Delete {
        /// Resource type.
        resource: String,
        /// Resource ID.
        id: String,
        /// Skip confirmation.
        #[arg(long = "yes", short = 'y')]
        yes: bool,
    },

    /// Check server status.
    Status,

    /// Show version.
    Version,
}

#[derive(Subcommand, Debug)]
enum ContextAction {
    /// Create a new context.
    Create {
        /// Context name.
        name: String,
        /// Server config directory (default: /etc/openerp).
        #[arg(long, default_value = "/etc/openerp")]
        config_dir: String,
        /// Data directory (default: /var/lib/openerp/<name>).
        #[arg(long)]
        data_dir: Option<String>,
        /// Root password (non-interactive, for CI/automation).
        /// If not provided, will prompt interactively.
        #[arg(long)]
        password: Option<String>,
    },
    /// List all contexts.
    List,
    /// Set properties on a context.
    Set {
        name: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Delete a context.
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
enum UseWhat {
    /// Switch to a context.
    Context { name: String },
}

#[derive(Subcommand, Debug)]
enum RootAction {
    /// Change root password.
    Chpwd {
        /// Target context (default: current).
        #[arg(long)]
        context: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config_path = cli
        .config
        .map(std::path::PathBuf::from)
        .unwrap_or_else(config::ClientConfig::default_path);

    match cli.command {
        Commands::Context { action } => match action {
            ContextAction::Create {
                name,
                config_dir,
                data_dir,
                password,
            } => {
                let data_dir = data_dir.unwrap_or_else(|| {
                    format!("/var/lib/openerp/{}", name)
                });

                let password = if let Some(p) = password {
                    // Non-interactive mode (CI/automation).
                    if p.is_empty() {
                        anyhow::bail!("Password cannot be empty.");
                    }
                    p
                } else {
                    // Interactive mode.
                    let pw = rpassword::prompt_password("Enter root password: ")?;
                    let confirm = rpassword::prompt_password("Confirm root password: ")?;
                    if pw != confirm {
                        anyhow::bail!("Passwords do not match.");
                    }
                    if pw.is_empty() {
                        anyhow::bail!("Password cannot be empty.");
                    }
                    pw
                };

                commands::context::create(
                    &name,
                    &config_dir,
                    &data_dir,
                    &password,
                    &config_path,
                )?;
            }
            ContextAction::List => {
                commands::context::list(&config_path)?;
            }
            ContextAction::Set { name, server } => {
                commands::context::set(&name, server.as_deref(), &config_path)?;
            }
            ContextAction::Delete { name } => {
                commands::context::delete(&name, &config_path)?;
            }
        },

        Commands::Use { what } => match what {
            UseWhat::Context { name } => {
                commands::context::use_context(&name, &config_path)?;
            }
        },

        Commands::Root { action } => match action {
            RootAction::Chpwd { context } => {
                let old = rpassword::prompt_password("Current root password: ")?;
                let new = rpassword::prompt_password("New root password: ")?;
                let confirm = rpassword::prompt_password("Confirm new password: ")?;
                if new != confirm {
                    anyhow::bail!("Passwords do not match.");
                }
                if new.is_empty() {
                    anyhow::bail!("Password cannot be empty.");
                }
                commands::root::chpwd(context.as_deref(), &old, &new, &config_path)?;
            }
        },

        Commands::Login { user, password } => {
            let username = user.unwrap_or_else(|| {
                eprint!("Username: ");
                let mut s = String::new();
                std::io::stdin().read_line(&mut s).unwrap();
                s.trim().to_string()
            });
            let password = password.unwrap_or_else(|| {
                rpassword::prompt_password("Password: ").unwrap_or_default()
            });
            commands::login::login(&username, &password, &config_path)?;
        }

        Commands::Logout => {
            commands::login::logout(&config_path)?;
        }

        Commands::Get {
            resource,
            id,
            limit,
            offset,
        } => {
            let json_output = cli.output == "json";
            commands::resource::get(
                &resource,
                id.as_deref(),
                json_output,
                limit,
                offset,
                &config_path,
            )?;
        }

        Commands::Create {
            resource,
            json_body,
            file,
        } => {
            let body = if let Some(path) = file {
                std::fs::read_to_string(&path)?
            } else if let Some(json) = json_body {
                json
            } else {
                anyhow::bail!("Provide --json or -f <file>.");
            };
            commands::resource::create(&resource, &body, &config_path)?;
        }

        Commands::Update {
            resource,
            id,
            json_body,
        } => {
            commands::resource::update(&resource, &id, &json_body, &config_path)?;
        }

        Commands::Delete { resource, id, yes } => {
            if !yes {
                eprint!("Are you sure? [y/N]: ");
                let mut s = String::new();
                std::io::stdin().read_line(&mut s).unwrap();
                if !s.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }
            commands::resource::delete(&resource, &id, &config_path)?;
        }

        Commands::Status => {
            commands::resource::status(&config_path)?;
        }

        Commands::Version => {
            println!("openerp cli v{}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
