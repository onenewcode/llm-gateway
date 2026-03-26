//! 后端节点定义
//!
//! 后端节点代表实际的服务实例，支持：
//! - 简单 URL 转发
//! - 协议特定的 URL 映射
//! - API Key 替换

use std::collections::HashMap;

/// 后端节点结构
///
/// # 字段
///
/// * `base_url` - 基础 URL 配置，支持多协议映射
/// * `api_key` - 可选的 API Key，用于替换请求中的密钥
#[derive(Clone, Debug)]
pub struct BackendNode {
    pub base_url: BaseUrl,
    pub api_key: Option<String>,
}

/// 基础 URL 配置
///
/// 支持两种配置方式：
/// 1. 单一 URL：所有协议使用同一个 URL
/// 2. 多协议映射：不同协议使用不同的 URL
///
/// # 字段
///
/// * `map` - 协议到 URL 的映射表
/// * `default` - 默认 URL，当协议未在 map 中时使用
#[derive(Clone, Debug)]
pub enum BaseUrl {
    Multi(HashMap<String, String>),
    AllInOne(String),
}

pub enum UrlResult<'a> {
    Native(&'a str),
    Foreign(&'a str, &'a str),
    Empty,
}

impl BaseUrl {
    /// 根据协议获取对应的 URL
    ///
    /// # 参数
    ///
    /// * `protocol` - 协议名称，如 "anthropic", "openai" 等
    ///
    /// # 返回值
    ///
    /// 如果协议在 map 中，返回对应的 URL；否则返回 None
    pub fn get(&self, protocol: &str) -> UrlResult<'_> {
        match self {
            Self::Multi(map) => map
                .get(protocol)
                .map(|s| UrlResult::Native(s.as_str()))
                .or_else(|| {
                    map.iter()
                        .next()
                        .map(|(protocol, url)| UrlResult::Foreign(protocol.as_str(), url.as_str()))
                })
                .unwrap_or(UrlResult::Empty),
            Self::AllInOne(url) => UrlResult::Native(url),
        }
    }
}
