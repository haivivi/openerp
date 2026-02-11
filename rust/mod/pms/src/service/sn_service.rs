use std::collections::HashMap;

use openerp_core::ServiceError;
use kv::KVStore;

use crate::model::SNSegment;
use crate::sn::{
    default_config, decode_sn, encode_sn, DecodeOutput, EncodeInput, EncodeOutput, SNConfig,
};
use super::PmsService;

impl PmsService {
    /// Get the SN configuration from the KV store, or return the default.
    pub fn get_sn_config(&self) -> Result<SNConfig, ServiceError> {
        match self.kv.get("config:sn") {
            Ok(Some(data)) => {
                serde_json::from_slice(&data).map_err(|e| ServiceError::Internal(e.to_string()))
            }
            Ok(None) => Ok(default_config()),
            Err(e) => Err(ServiceError::Storage(e.to_string())),
        }
    }

    /// Update the SN configuration in the KV store.
    pub fn set_sn_config(&self, config: &SNConfig) -> Result<(), ServiceError> {
        config.validate()
            .map_err(|e| ServiceError::Validation(e.to_string()))?;
        let data = serde_json::to_vec(config)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.kv.set("config:sn", &data)
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    /// Encode a serial number from the given parameters.
    pub fn encode_serial_number(
        &self,
        model_no: u32,
        dimensions: HashMap<String, u32>,
        timestamp: Option<(u32, u32)>,
    ) -> Result<EncodeOutput, ServiceError> {
        let config = self.get_sn_config()?;
        let input = EncodeInput {
            model_no,
            dimensions,
            timestamp,
        };
        encode_sn(&config, &input).map_err(|e| ServiceError::Validation(e.to_string()))
    }

    /// Decode a serial number string.
    pub fn decode_serial_number(&self, sn: &str) -> Result<DecodeOutput, ServiceError> {
        let config = self.get_sn_config()?;
        decode_sn(&config, sn).map_err(|e| ServiceError::Validation(e.to_string()))
    }

    /// List all SN segments, optionally filtered by dimension.
    pub fn list_sn_segments(
        &self,
        dimension: Option<&str>,
    ) -> Result<Vec<SNSegment>, ServiceError> {
        let prefix = match dimension {
            Some(dim) => format!("config:segment:{}:", dim),
            None => "config:segment:".to_string(),
        };

        let entries = self.kv.scan(&prefix)
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let mut segments = Vec::new();
        for (_key, value) in entries {
            if let Ok(seg) = serde_json::from_slice::<SNSegment>(&value) {
                segments.push(seg);
            }
        }

        Ok(segments)
    }

    /// Get available SN dimensions (unique dimension names).
    pub fn list_sn_dimensions(&self) -> Result<Vec<String>, ServiceError> {
        let entries = self.kv.scan("config:segment:")
            .map_err(|e| ServiceError::Storage(e.to_string()))?;

        let mut dims = std::collections::BTreeSet::new();
        for (key, _) in entries {
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() >= 3 {
                dims.insert(parts[2].to_string());
            }
        }

        Ok(dims.into_iter().collect())
    }

    /// Create or update an SN segment.
    pub fn upsert_sn_segment(&self, segment: &SNSegment) -> Result<(), ServiceError> {
        let key = format!(
            "config:segment:{}:{}",
            segment.dimension,
            segment.name.to_lowercase().replace(' ', "_")
        );
        let data = serde_json::to_vec(segment)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        self.kv.set(&key, &data)
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }

    /// Delete an SN segment.
    pub fn delete_sn_segment(
        &self,
        dimension: &str,
        name: &str,
    ) -> Result<(), ServiceError> {
        let key = format!(
            "config:segment:{}:{}",
            dimension,
            name.to_lowercase().replace(' ', "_")
        );
        self.kv.delete(&key)
            .map_err(|e| ServiceError::Storage(e.to_string()))
    }
}
