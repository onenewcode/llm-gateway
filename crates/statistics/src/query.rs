//! 查询参数和结果类型定义

use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

/// 原始事件查询过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilter {
    /// 起始时间（毫秒时间戳）
    pub start_time: Option<i64>,
    /// 结束时间（毫秒时间戳）
    pub end_time: Option<i64>,
    /// 模型名称
    pub model: Option<String>,
    /// 后端节点
    pub backend: Option<String>,
    /// 成功/失败
    pub success: Option<bool>,
    /// 限制返回数量
    pub limit: Option<usize>,
    /// 偏移量
    pub offset: Option<usize>,
}

/// 聚合查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggQuery {
    /// 起始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 窗口大小（秒）
    pub window_size: NonZeroU64,
    /// 模型名称
    pub model: Option<String>,
    /// 后端节点
    pub backend: Option<String>,
}

/// 聚合统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggStats {
    /// 窗口起始时间（毫秒时间戳）
    pub window_start: u64,
    /// 窗口大小（毫秒）
    pub window_size: u64,
    /// 模型名称
    pub model: String,
    /// 后端节点
    pub backend: String,
    /// 总请求数
    pub total_requests: i64,
    /// 成功数
    pub success_count: i64,
    /// 失败数
    pub fail_count: i64,
    /// 平均耗时（毫秒）
    pub avg_duration_ms: i64,
    /// 最小耗时（毫秒）
    pub min_duration_ms: i64,
    /// 最大耗时（毫秒）
    pub max_duration_ms: i64,
    /// P50 延迟（毫秒）
    pub p50_duration_ms: Option<i64>,
    /// P90 延迟（毫秒）
    pub p90_duration_ms: Option<i64>,
    /// P99 延迟（毫秒）
    pub p99_duration_ms: Option<i64>,
}

/// Result of aggregation with limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResult {
    /// The aggregated statistics
    pub stats: Vec<AggStats>,
    /// Summary object for pagination/early termination
    pub summary: AggSummary,
}

/// Summary object indicating completion status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggSummary {
    /// Window start time (RFC3339 format in response)
    pub window_start: u64,
    /// Remaining seconds not computed (0 if finished)
    pub window_size_seconds: u64,
    /// Reason for stopping
    pub stop_reason: String,
}

impl AggSummary {
    pub fn finished(end_time: u64) -> Self {
        Self {
            window_start: end_time,
            window_size_seconds: 0,
            stop_reason: "finished".to_string(),
        }
    }

    pub fn too_many_data(current_time: u64, end_time: u64) -> Self {
        Self {
            window_start: current_time,
            window_size_seconds: end_time.saturating_sub(current_time).div_ceil(1000),
            stop_reason: "too_many_data".to_string(),
        }
    }
}

/// Parse time string like "30s", "5m", "1h", "7d" or plain number (seconds)
/// Supports units: s (seconds), m/min (minutes), h (hours), d (days)
/// Returns seconds as u64. Default to 1h (3600s) if input is empty.
pub fn parse_time(value: &str) -> Result<u64, String> {
    let value = value.trim();

    fn parse_num(num: &str, sec: u64) -> Result<u64, String> {
        num.parse::<u64>()
            .map_err(|_| format!("Invalid number in duration: {num}"))
            .and_then(|n| {
                n.checked_mul(sec)
                    .ok_or_else(|| format!("Duration overflow: {num}"))
            })
    }

    // Default to 1h if empty
    if value.is_empty() {
        Ok(3600)
    }
    // Try to parse as plain number (seconds)
    else if let Ok(secs) = value.parse() {
        Ok(secs)
    }
    // Try to strip each unit suffix
    else if let Some(num_str) = value.strip_suffix('s') {
        parse_num(num_str, 1)
    } else if let Some(num_str) = value.strip_suffix("min") {
        parse_num(num_str, 60)
    } else if let Some(num_str) = value.strip_suffix('m') {
        parse_num(num_str, 60)
    } else if let Some(num_str) = value.strip_suffix('h') {
        parse_num(num_str, 60 * 60)
    } else if let Some(num_str) = value.strip_suffix('d') {
        parse_num(num_str, 60 * 60 * 24)
    } else {
        Err(format!("Unknown time unit in: {value}"))
    }
}

/// 统计查询构建器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsQueryBuilder {
    start_time: u64,
    end_time: u64,
    window_size_secs: u64,
    model: Option<String>,
    backend: Option<String>,
}

impl StatsQueryBuilder {
    /// 创建新的查询构建器
    pub fn new(start_time: u64, end_time: u64, window_size_secs: u64) -> Self {
        Self {
            start_time,
            end_time,
            window_size_secs,
            model: None,
            backend: None,
        }
    }

    /// 添加模型过滤条件
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 添加后端过滤条件
    pub fn with_backend(mut self, backend: impl Into<String>) -> Self {
        self.backend = Some(backend.into());
        self
    }

    /// 构建查询参数
    pub fn build(self) -> AggQuery {
        let window_size = NonZeroU64::new(self.window_size_secs)
            .unwrap_or_else(|| NonZeroU64::new(3600).unwrap());
        AggQuery {
            start_time: self.start_time,
            end_time: self.end_time,
            window_size,
            model: self.model,
            backend: self.backend,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_seconds() {
        assert_eq!(parse_time("30s"), Ok(30));
        assert_eq!(parse_time("100s"), Ok(100));
    }

    #[test]
    fn test_parse_time_minutes() {
        assert_eq!(parse_time("5m"), Ok(300));
        assert_eq!(parse_time("10min"), Ok(600));
    }

    #[test]
    fn test_parse_time_hours() {
        assert_eq!(parse_time("1h"), Ok(3600));
        assert_eq!(parse_time("24h"), Ok(86400));
    }

    #[test]
    fn test_parse_time_days() {
        assert_eq!(parse_time("1d"), Ok(86400));
        assert_eq!(parse_time("7d"), Ok(604800));
    }

    #[test]
    fn test_parse_time_plain_number() {
        assert_eq!(parse_time("3600"), Ok(3600));
        assert_eq!(parse_time("100"), Ok(100));
    }

    #[test]
    fn test_parse_time_empty() {
        assert_eq!(parse_time(""), Ok(3600)); // Default to 1h
        assert_eq!(parse_time("   "), Ok(3600)); // Whitespace trimmed
    }

    #[test]
    fn test_parse_time_invalid() {
        assert!(parse_time("abc").is_err());
        assert!(parse_time("1x").is_err());
        assert!(parse_time("m5").is_err());
    }

    #[test]
    fn test_parse_time_overflow() {
        // u64::MAX / 86400 + 1 days would overflow
        assert!(parse_time("300000000000000000d").is_err());
        // Large seconds that would overflow when multiplied
        assert!(parse_time("18446744073709551616s").is_err());
    }

    #[test]
    fn test_agg_summary_subsecond() {
        // 500ms remaining should be 1 second (ceil)
        let summary = AggSummary::too_many_data(0, 500);
        assert_eq!(summary.window_size_seconds, 1);

        // Exactly 1s
        let summary = AggSummary::too_many_data(0, 1000);
        assert_eq!(summary.window_size_seconds, 1);

        // 1.5s should be 2s (ceil)
        let summary = AggSummary::too_many_data(0, 1500);
        assert_eq!(summary.window_size_seconds, 2);

        // 0ms remaining should be 0
        let summary = AggSummary::too_many_data(1000, 1000);
        assert_eq!(summary.window_size_seconds, 0);
    }

    #[test]
    fn test_stats_query_builder() {
        let query = StatsQueryBuilder::new(1000, 2000, 3600)
            .with_model("qwen3.5-35b")
            .with_backend("sglang")
            .build();

        assert_eq!(query.start_time, 1000);
        assert_eq!(query.end_time, 2000);
        assert_eq!(query.window_size, NonZeroU64::new(3600).unwrap());
        assert_eq!(query.model, Some("qwen3.5-35b".to_string()));
        assert_eq!(query.backend, Some("sglang".to_string()));
    }

    #[test]
    fn test_event_filter_default() {
        let filter = EventFilter::default();
        assert!(filter.start_time.is_none());
        assert!(filter.end_time.is_none());
        assert!(filter.model.is_none());
        assert!(filter.backend.is_none());
        assert!(filter.success.is_none());
        assert!(filter.limit.is_none());
        assert!(filter.offset.is_none());
    }
}
