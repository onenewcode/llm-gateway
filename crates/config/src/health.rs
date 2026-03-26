//! Health monitoring configuration for backend nodes.
//!
//! This module provides configuration for health monitoring of backend nodes,
//! including sliding window failure tracking and cooldown periods.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Health monitoring configuration (external/TOML format).
///
/// This struct is used for deserializing configuration from TOML files.
/// It uses primitive types (`u64`, `u32`) for serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Size of the sliding time window in seconds (default: 60)
    #[serde(default = "default_window_size")]
    pub window_size: u64,

    /// Number of failures that trigger cooldown (default: 3)
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// Duration of cooldown period in seconds (default: 300)
    #[serde(default = "default_cooldown_duration")]
    pub cooldown_duration: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            window_size: default_window_size(),
            failure_threshold: default_failure_threshold(),
            cooldown_duration: default_cooldown_duration(),
        }
    }
}

fn default_window_size() -> u64 {
    60
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_cooldown_duration() -> u64 {
    300
}

impl HealthConfig {
    /// Convert to internal configuration with Duration types.
    pub fn to_internal(&self) -> InternalHealthConfig {
        InternalHealthConfig {
            window_size: Duration::from_secs(self.window_size),
            failure_threshold: self.failure_threshold,
            cooldown_duration: Duration::from_secs(self.cooldown_duration),
        }
    }
}

/// Internal health monitoring configuration.
///
/// This struct uses `Duration` types for internal use within the application.
/// It is converted from `HealthConfig` after deserialization.
#[derive(Debug, Clone)]
pub struct InternalHealthConfig {
    /// Size of the sliding time window
    pub window_size: Duration,

    /// Number of failures that trigger cooldown
    pub failure_threshold: u32,

    /// Duration of cooldown period
    pub cooldown_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test default HealthConfig values
    #[test]
    fn test_health_config_default_values() {
        let config = HealthConfig::default();
        assert_eq!(config.window_size, 60);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.cooldown_duration, 300);
    }

    /// Test HealthConfig to InternalHealthConfig conversion
    #[test]
    fn test_health_config_to_internal() {
        let config = HealthConfig {
            window_size: 120,
            failure_threshold: 5,
            cooldown_duration: 600,
        };

        let internal = config.to_internal();

        assert_eq!(internal.window_size, Duration::from_secs(120));
        assert_eq!(internal.failure_threshold, 5);
        assert_eq!(internal.cooldown_duration, Duration::from_secs(600));
    }

    /// Test deserializing full HealthConfig from TOML
    #[test]
    fn test_health_config_from_toml_full() {
        let toml_str = r#"
window_size = 120
failure_threshold = 5
cooldown_duration = 600
"#;

        let config: HealthConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.window_size, 120);
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.cooldown_duration, 600);
    }

    /// Test deserializing HealthConfig with default values
    #[test]
    fn test_health_config_from_toml_defaults() {
        let toml_str = r#"
# Empty config should use defaults
"#;

        let config: HealthConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.window_size, 60);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.cooldown_duration, 300);
    }

    /// Test partial HealthConfig from TOML (some defaults)
    #[test]
    fn test_health_config_from_toml_partial() {
        let toml_str = r#"
failure_threshold = 10
"#;

        let config: HealthConfig = toml::from_str(toml_str).unwrap();

        assert_eq!(config.window_size, 60); // default
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.cooldown_duration, 300); // default
    }

    /// Test InternalHealthConfig default conversion
    #[test]
    fn test_internal_health_config_from_default() {
        let config = HealthConfig::default();
        let internal = config.to_internal();

        assert_eq!(internal.window_size, Duration::from_secs(60));
        assert_eq!(internal.failure_threshold, 3);
        assert_eq!(internal.cooldown_duration, Duration::from_secs(300));
    }
}
