//! 输入节点定义
//!
//! 输入节点是网关的入口点，负责：
//! - 监听指定端口
//! - 接收客户端请求
//! - 根据模型名称路由到对应的虚拟节点或后端

/// 输入节点结构
///
/// # 字段
///
/// * `name` - 节点名称，用于路由识别
/// * `port` - 监听端口号
/// * `models` - 支持的模型列表，请求将根据模型名路由
#[derive(Clone, Debug)]
pub struct InputNode {
    pub port: u16,
    pub models: Vec<String>,
}

#[cfg(test)]
mod tests {
    use crate::GatewayConfig;
    use std::str::FromStr;

    /// 测试基本输入节点解析
    #[test]
    fn test_parse_input_node() {
        let toml_str = r#"
[input.service]
port = 8000
models = ["qwen3.5-35b-a3b", "kimi-k2.5"]
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.nodes.contains_key("service"));
        match &config.nodes["service"] {
            crate::Node::Input(input) => {
                assert_eq!(input.port, 8000);
                assert_eq!(input.models, vec!["qwen3.5-35b-a3b", "kimi-k2.5"]);
            }
            _ => panic!("Expected Input node"),
        }
    }

    /// 测试多模型列表解析
    #[test]
    fn test_parse_input_node_multiple_models() {
        let toml_str = r#"
[input.service]
port = 8000
models = ["qwen3.5-35b-a3b", "qwen3.5-122b-a10b", "kimi-k2.5"]
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        match &config.nodes["service"] {
            crate::Node::Input(input) => {
                assert_eq!(input.models.len(), 3);
            }
            _ => panic!("Expected Input node"),
        }
    }
}
