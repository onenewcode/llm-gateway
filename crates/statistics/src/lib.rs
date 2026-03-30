//! LLM Gateway 事件统计模块
//!
//! 提供请求路由事件的记录、查询和聚合统计功能

pub mod aggregator;
pub mod cli;
pub mod config;
pub mod event;
pub mod query;
pub mod sqlite;
pub mod store;

// 重新导出常用类型
pub use aggregator::Aggregator;
pub use cli::{OutputFormat, format_events};
pub use config::StatisticsConfig;
pub use event::{RoutingEvent, RoutingEventBuilder};
pub use query::{AggQuery, AggStats, EventFilter, StatsQueryBuilder, TimeGranularity};
pub use sqlite::SqliteStore;
pub use store::StatsStoreManager;

/// 统计模块错误类型
#[derive(Debug, thiserror::Error)]
pub enum StatisticsError {
    /// 数据库错误
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// 配置错误
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// 查询错误
    #[error("Query error: {0}")]
    QueryError(String),
}

impl From<String> for StatisticsError {
    fn from(s: String) -> Self {
        StatisticsError::ConfigurationError(s)
    }
}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, StatisticsError>;
