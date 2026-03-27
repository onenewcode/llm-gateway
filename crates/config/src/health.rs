//! 健康监控配置模块
//!
//! 提供后端节点健康监控的配置，包括滑动窗口失败追踪和冷却期设置

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 健康监控配置（外部/TOML 格式）
///
/// 用于从 TOML 文件反序列化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// 滑动时间窗口大小（秒），默认 60
    #[serde(default = "default_window_size")]
    pub window_size: u64,

    /// 触发冷却期的失败次数，默认 3
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// 冷却期持续时间（秒），默认 300
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

/// 默认滑动窗口大小：60 秒
fn default_window_size() -> u64 {
    60
}

/// 默认失败阈值：3 次
fn default_failure_threshold() -> u32 {
    3
}

/// 默认冷却期：300 秒
fn default_cooldown_duration() -> u64 {
    300
}

impl HealthConfig {
    /// 转换为内部配置（使用 Duration 类型）
    pub fn to_internal(&self) -> InternalHealthConfig {
        InternalHealthConfig {
            window_size: Duration::from_secs(self.window_size),
            failure_threshold: self.failure_threshold,
            cooldown_duration: Duration::from_secs(self.cooldown_duration),
        }
    }
}

/// 内部健康监控配置
///
/// 使用 Duration 类型用于应用内部
#[derive(Debug, Clone)]
pub struct InternalHealthConfig {
    /// 滑动时间窗口大小
    pub window_size: Duration,
    /// 触发冷却期的失败次数
    pub failure_threshold: u32,
    /// 冷却期持续时间
    pub cooldown_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试默认配置值
    #[test]
    fn test_health_config_default_values() {
        let config = HealthConfig::default();
        assert_eq!(config.window_size, 60);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.cooldown_duration, 300);
    }

    /// 测试 HealthConfig 到 InternalHealthConfig 的转换
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

    /// 测试从 TOML 完整解析配置
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

    /// 测试空配置使用默认值
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

    /// 测试部分配置（部分使用默认值）
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
