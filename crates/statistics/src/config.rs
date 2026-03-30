//! 统计配置模块

use serde::{Deserialize, Serialize};

/// 统计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsConfig {
    /// 是否启用统计
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 数据库文件路径
    #[serde(default = "default_db_path")]
    pub db_path: String,
    /// 数据保留天数（自动清理）
    #[serde(default = "default_retention_days")]
    pub retention_days: u64,
    /// 写入缓冲区大小（事件数量）
    #[serde(default = "default_write_buffer_size")]
    pub write_buffer_size: usize,
}

impl Default for StatisticsConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            db_path: default_db_path(),
            retention_days: default_retention_days(),
            write_buffer_size: default_write_buffer_size(),
        }
    }
}

/// 默认启用统计
fn default_enabled() -> bool {
    true
}

/// 默认数据库路径
fn default_db_path() -> String {
    "stats.db".to_string()
}

/// 默认保留天数
fn default_retention_days() -> u64 {
    7
}

/// 默认写入缓冲区大小
fn default_write_buffer_size() -> usize {
    1000
}

impl StatisticsConfig {
    /// 创建内存配置（用于测试）
    pub fn in_memory() -> Self {
        Self {
            enabled: true,
            db_path: ":memory:".to_string(),
            retention_days: 7,
            write_buffer_size: 100,
        }
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.retention_days == 0 {
            return Err("retention_days must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StatisticsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.db_path, "stats.db");
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn test_in_memory_config() {
        let config = StatisticsConfig::in_memory();
        assert!(config.enabled);
        assert_eq!(config.db_path, ":memory:");
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = StatisticsConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_retention() {
        let mut config = StatisticsConfig::default();
        config.retention_days = 0;
        assert!(config.validate().is_err());
    }
}
