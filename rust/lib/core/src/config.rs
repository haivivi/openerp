use std::path::PathBuf;

/// Common CLI configuration shared by all services.
///
/// Each service binary parses these from command-line arguments or environment
/// variables, then passes them to storage layer initialization.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Directory containing static configuration files (YAML/JSON).
    /// These are loaded into the KV file layer (read-only).
    pub data_dir: Option<PathBuf>,

    /// Path to the redb database file.
    /// Defaults to `{data_dir}/data.redb` if not specified.
    pub db_path: Option<PathBuf>,

    /// Path to the SQLite database file.
    /// Defaults to `{data_dir}/data.sqlite` if not specified.
    pub sqlite_path: Option<PathBuf>,

    /// Directory for tantivy search indexes.
    /// Defaults to `{data_dir}/search/` if not specified.
    pub search_dir: Option<PathBuf>,

    /// Directory for blob storage.
    /// Defaults to `{data_dir}/blobs/` if not specified.
    pub blob_dir: Option<PathBuf>,

    /// Directory for tsdb WAL and archive.
    /// Defaults to `{data_dir}/tsdb/` if not specified.
    pub tsdb_dir: Option<PathBuf>,

    /// Listen address for the HTTP server.
    pub listen: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            db_path: None,
            sqlite_path: None,
            search_dir: None,
            blob_dir: None,
            tsdb_dir: None,
            listen: "0.0.0.0:8080".to_string(),
        }
    }
}

impl ServiceConfig {
    /// Parse configuration from command-line arguments.
    ///
    /// Supported flags:
    /// - `--data-dir=PATH`
    /// - `--db=PATH`
    /// - `--sqlite=PATH`
    /// - `--search-dir=PATH`
    /// - `--blob-dir=PATH`
    /// - `--tsdb-dir=PATH`
    /// - `--listen=ADDR`
    pub fn from_args(args: &[String]) -> Self {
        let mut config = ServiceConfig::default();

        for arg in args {
            if let Some(val) = arg.strip_prefix("--data-dir=") {
                config.data_dir = Some(PathBuf::from(val));
            } else if let Some(val) = arg.strip_prefix("--db=") {
                config.db_path = Some(PathBuf::from(val));
            } else if let Some(val) = arg.strip_prefix("--sqlite=") {
                config.sqlite_path = Some(PathBuf::from(val));
            } else if let Some(val) = arg.strip_prefix("--search-dir=") {
                config.search_dir = Some(PathBuf::from(val));
            } else if let Some(val) = arg.strip_prefix("--blob-dir=") {
                config.blob_dir = Some(PathBuf::from(val));
            } else if let Some(val) = arg.strip_prefix("--tsdb-dir=") {
                config.tsdb_dir = Some(PathBuf::from(val));
            } else if let Some(val) = arg.strip_prefix("--listen=") {
                config.listen = val.to_string();
            }
        }

        config
    }

    /// Resolve the redb database path, falling back to `{data_dir}/data.redb`.
    pub fn resolve_db_path(&self) -> PathBuf {
        self.db_path
            .clone()
            .unwrap_or_else(|| self.resolve_data_subpath("data.redb"))
    }

    /// Resolve the SQLite database path, falling back to `{data_dir}/data.sqlite`.
    pub fn resolve_sqlite_path(&self) -> PathBuf {
        self.sqlite_path
            .clone()
            .unwrap_or_else(|| self.resolve_data_subpath("data.sqlite"))
    }

    /// Resolve the search index directory.
    pub fn resolve_search_dir(&self) -> PathBuf {
        self.search_dir
            .clone()
            .unwrap_or_else(|| self.resolve_data_subpath("search"))
    }

    /// Resolve the blob storage directory.
    pub fn resolve_blob_dir(&self) -> PathBuf {
        self.blob_dir
            .clone()
            .unwrap_or_else(|| self.resolve_data_subpath("blobs"))
    }

    /// Resolve the tsdb directory.
    pub fn resolve_tsdb_dir(&self) -> PathBuf {
        self.tsdb_dir
            .clone()
            .unwrap_or_else(|| self.resolve_data_subpath("tsdb"))
    }

    fn resolve_data_subpath(&self, name: &str) -> PathBuf {
        self.data_dir
            .as_ref()
            .map(|d| d.join(name))
            .unwrap_or_else(|| PathBuf::from(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_args() {
        let args = vec![
            "--data-dir=/tmp/openerp".to_string(),
            "--listen=127.0.0.1:9090".to_string(),
        ];
        let config = ServiceConfig::from_args(&args);
        assert_eq!(config.data_dir, Some(PathBuf::from("/tmp/openerp")));
        assert_eq!(config.listen, "127.0.0.1:9090");
    }

    #[test]
    fn test_resolve_defaults() {
        let config = ServiceConfig {
            data_dir: Some(PathBuf::from("/data")),
            ..Default::default()
        };
        assert_eq!(config.resolve_db_path(), PathBuf::from("/data/data.redb"));
        assert_eq!(
            config.resolve_sqlite_path(),
            PathBuf::from("/data/data.sqlite")
        );
        assert_eq!(
            config.resolve_search_dir(),
            PathBuf::from("/data/search")
        );
    }
}
