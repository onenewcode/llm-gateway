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
pub struct BaseUrl {
    pub map: HashMap<String, String>,
    pub default: String,
}

impl BaseUrl {
    /// 获取默认 URL
    pub fn default(&self) -> &str {
        &self.default
    }

    /// 根据协议获取对应的 URL
    ///
    /// # 参数
    ///
    /// * `protocol` - 协议名称，如 "anthropic", "openai" 等
    ///
    /// # 返回值
    ///
    /// 如果协议在 map 中，返回对应的 URL；否则返回 None
    pub fn get(&self, protocol: &str) -> Option<&str> {
        self.map.get(protocol).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::{GatewayConfig, Node};
    use std::str::FromStr;

    /// 测试简单后端解析（[backend] 下的键值对）
    #[test]
    fn test_parse_simple_backend() {
        let toml_str = r#"
[backend]
"sglang-qwen3.5-35b-a3b" = "http://172.17.250.163:30001"
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.nodes.contains_key("sglang-qwen3.5-35b-a3b"));
        match &config.nodes["sglang-qwen3.5-35b-a3b"] {
            Node::Backend(backend) => {
                assert_eq!(backend.base_url.default(), "http://172.17.250.163:30001");
                assert!(backend.api_key.is_none());
            }
            _ => panic!("Expected Backend node"),
        }
    }

    /// 测试协议特定 URL 解析
    #[test]
    fn test_parse_backend_with_protocol_specific_urls() {
        let toml_str = r#"
[backend.aliyun]
base-url = { anthropic = "https://dashscope.aliyuncs.com/apps/anthropic" }
api-key = "$ALIYUN_API_KEY"
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.nodes.contains_key("aliyun"));
        match &config.nodes["aliyun"] {
            Node::Backend(backend) => {
                assert_eq!(
                    backend.base_url.get("anthropic"),
                    Some("https://dashscope.aliyuncs.com/apps/anthropic")
                );
                assert_eq!(backend.api_key, Some("$ALIYUN_API_KEY".to_string()));
            }
            _ => panic!("Expected Backend node"),
        }
    }

    /// 测试带 default 的协议特定 URL 解析
    #[test]
    fn test_parse_backend_with_default_url() {
        let toml_str = r#"
[backend.aliyun]
base-url = { default = "https://default.url", anthropic = "https://anthropic.url" }
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        match &config.nodes["aliyun"] {
            Node::Backend(backend) => {
                assert_eq!(backend.base_url.default(), "https://default.url");
                assert_eq!(
                    backend.base_url.get("anthropic"),
                    Some("https://anthropic.url")
                );
            }
            _ => panic!("Expected Backend node"),
        }
    }
}
