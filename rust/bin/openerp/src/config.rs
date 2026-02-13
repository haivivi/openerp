//! Client-side context management.
//!
//! Reads/writes `~/.openerp/config.toml`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A single context — connection to an openerpd instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Context name (e.g. "cn-stage").
    pub name: String,

    /// Path to the server-side config file (for local deployments).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub config_path: String,

    /// Server URL (e.g. "http://localhost:8080").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub server: String,

    /// JWT token (set by `openerp login`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token: String,
}

/// Client configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Name of the currently active context.
    #[serde(rename = "current-context", default)]
    pub current_context: String,

    /// List of configured contexts.
    #[serde(default)]
    pub contexts: Vec<Context>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            current_context: String::new(),
            contexts: Vec::new(),
        }
    }
}

impl ClientConfig {
    /// Default config file path: ~/.openerp/config.toml.
    pub fn default_path() -> PathBuf {
        dirs_path().join("config.toml")
    }

    /// Load config from disk, or return default if file doesn't exist.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: ClientConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the currently active context, if any.
    pub fn current(&self) -> Option<&Context> {
        self.contexts.iter().find(|c| c.name == self.current_context)
    }

    /// Get a mutable reference to a context by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Context> {
        self.contexts.iter_mut().find(|c| c.name == name)
    }

    /// Add or update a context.
    pub fn upsert_context(&mut self, ctx: Context) {
        if let Some(existing) = self.get_mut(&ctx.name) {
            *existing = ctx;
        } else {
            self.contexts.push(ctx);
        }
    }

    /// Remove a context by name. Returns true if it was found.
    pub fn remove_context(&mut self, name: &str) -> bool {
        let len = self.contexts.len();
        self.contexts.retain(|c| c.name != name);
        if self.current_context == name {
            self.current_context = String::new();
        }
        self.contexts.len() < len
    }
}

/// Return the OpenERP config directory (~/.openerp).
fn dirs_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".openerp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert!(config.current_context.is_empty());
        assert!(config.contexts.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let mut config = ClientConfig::default();
        config.current_context = "test".to_string();
        config.contexts.push(Context {
            name: "test".to_string(),
            config_path: "/etc/openerp/test.toml".to_string(),
            server: "http://localhost:8080".to_string(),
            token: String::new(),
        });

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let back: ClientConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.current_context, "test");
        assert_eq!(back.contexts.len(), 1);
        assert_eq!(back.contexts[0].server, "http://localhost:8080");
    }
}
