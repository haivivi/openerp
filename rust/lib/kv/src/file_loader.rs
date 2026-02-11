use std::fs;
use std::path::Path;

use tracing::{debug, warn};

use crate::error::KVError;
use crate::overlay::OverlayKV;
use crate::traits::KVStore;

/// FileLoader scans a data directory and populates the file layer of an
/// OverlayKV. The directory structure determines the key namespace:
///
/// ```text
/// data-dir/
/// ├── models/h106.yaml          → config:model:h106
/// ├── models/h2xx.yaml          → config:model:h2xx
/// ├── segments/manufacturer/foxconn.yaml → config:segment:manufacturer:foxconn
/// ├── segments/channel/tmall.yaml       → config:segment:channel:tmall
/// ├── firmwares/h106/latest.yaml        → config:firmware:h106
/// └── sn-config.yaml                    → config:sn
/// ```
///
/// All loaded entries become read-only in the overlay.
pub struct FileLoader;

impl FileLoader {
    /// Load all YAML files from `data_dir` into the overlay's file layer.
    /// Returns the number of entries loaded.
    pub fn load<DB: KVStore>(
        data_dir: &Path,
        overlay: &OverlayKV<DB>,
    ) -> Result<usize, KVError> {
        if !data_dir.is_dir() {
            debug!("FileLoader: data dir {:?} does not exist, skipping", data_dir);
            return Ok(0);
        }

        let mut count = 0;

        // Top-level YAML files (e.g. sn-config.yaml → config:sn)
        count += Self::load_top_level(data_dir, overlay)?;

        // models/ directory
        let models_dir = data_dir.join("models");
        if models_dir.is_dir() {
            count += Self::load_directory(&models_dir, "config:model:", overlay)?;
        }

        // segments/ directory (two levels: segments/{dimension}/{name}.yaml)
        let segments_dir = data_dir.join("segments");
        if segments_dir.is_dir() {
            count += Self::load_segments(&segments_dir, overlay)?;
        }

        // firmwares/ directory (firmwares/{model}/latest.yaml)
        let firmwares_dir = data_dir.join("firmwares");
        if firmwares_dir.is_dir() {
            count += Self::load_firmwares(&firmwares_dir, overlay)?;
        }

        debug!("FileLoader: loaded {} entries from {:?}", count, data_dir);
        Ok(count)
    }

    /// Load top-level YAML files: sn-config.yaml → config:sn
    fn load_top_level<DB: KVStore>(
        data_dir: &Path,
        overlay: &OverlayKV<DB>,
    ) -> Result<usize, KVError> {
        let mut count = 0;
        let entries =
            fs::read_dir(data_dir).map_err(|e| KVError::Storage(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| KVError::Storage(e.to_string()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !Self::is_yaml(&path) {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            // Map known top-level files.
            let key = match stem {
                "sn-config" => "config:sn".to_string(),
                other => format!("config:{}", other),
            };

            let data =
                fs::read(&path).map_err(|e| KVError::Storage(e.to_string()))?;
            overlay.insert_file_entry(key, data);
            count += 1;
        }

        Ok(count)
    }

    /// Load a flat directory of YAML files with a key prefix.
    /// E.g. models/h106.yaml with prefix "config:model:" → key "config:model:h106"
    fn load_directory<DB: KVStore>(
        dir: &Path,
        prefix: &str,
        overlay: &OverlayKV<DB>,
    ) -> Result<usize, KVError> {
        let mut count = 0;
        let entries =
            fs::read_dir(dir).map_err(|e| KVError::Storage(e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| KVError::Storage(e.to_string()))?;
            let path = entry.path();
            if !path.is_file() || !Self::is_yaml(&path) {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            let key = format!("{}{}", prefix, stem);
            let data =
                fs::read(&path).map_err(|e| KVError::Storage(e.to_string()))?;
            overlay.insert_file_entry(key, data);
            count += 1;
        }

        Ok(count)
    }

    /// Load segments: segments/{dimension}/{name}.yaml → config:segment:{dimension}:{name}
    fn load_segments<DB: KVStore>(
        segments_dir: &Path,
        overlay: &OverlayKV<DB>,
    ) -> Result<usize, KVError> {
        let mut count = 0;
        let dim_entries = fs::read_dir(segments_dir)
            .map_err(|e| KVError::Storage(e.to_string()))?;

        for dim_entry in dim_entries {
            let dim_entry =
                dim_entry.map_err(|e| KVError::Storage(e.to_string()))?;
            let dim_path = dim_entry.path();
            if !dim_path.is_dir() {
                continue;
            }

            let dimension = dim_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            let prefix = format!("config:segment:{}:", dimension);
            count += Self::load_directory(&dim_path, &prefix, overlay)?;
        }

        Ok(count)
    }

    /// Load firmwares: firmwares/{model}/latest.yaml → config:firmware:{model}
    fn load_firmwares<DB: KVStore>(
        firmwares_dir: &Path,
        overlay: &OverlayKV<DB>,
    ) -> Result<usize, KVError> {
        let mut count = 0;
        let model_entries = fs::read_dir(firmwares_dir)
            .map_err(|e| KVError::Storage(e.to_string()))?;

        for model_entry in model_entries {
            let model_entry =
                model_entry.map_err(|e| KVError::Storage(e.to_string()))?;
            let model_path = model_entry.path();
            if !model_path.is_dir() {
                continue;
            }

            let model_name = model_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            let latest = model_path.join("latest.yaml");
            if latest.is_file() {
                let key = format!("config:firmware:{}", model_name);
                let data = fs::read(&latest)
                    .map_err(|e| KVError::Storage(e.to_string()))?;
                overlay.insert_file_entry(key, data);
                count += 1;
            } else {
                warn!(
                    "FileLoader: no latest.yaml in firmwares/{}, skipping",
                    model_name
                );
            }
        }

        Ok(count)
    }

    fn is_yaml(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yaml") | Some("yml")
        )
    }
}
