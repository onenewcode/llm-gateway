//! 节点类型定义
//!
//! 定义网关中的节点类型，包括：
//! - [`Node`] - 节点枚举，包含所有节点类型
//! - [`VirtualNode`] - 虚拟节点，定义路由策略

use crate::{BackendNode, InputNode};

/// 节点类型枚举
///
/// 网关支持三种节点类型：
/// - [`Input`](Node::Input) - 输入节点，网关入口
/// - [`Virtual`](Node::Virtual) - 虚拟节点，定义路由策略
/// - [`Backend`](Node::Backend) - 后端节点，实际服务
#[derive(Clone, Debug)]
pub enum Node {
    Input(InputNode),
    Virtual(VirtualNode),
    Backend(BackendNode),
}

/// 虚拟节点类型
///
/// 虚拟节点用于定义路由策略，目前支持：
/// - [`Sequence`](VirtualNode::Sequence) - 顺序尝试，按列表顺序尝试后端
#[derive(Clone, Debug)]
pub enum VirtualNode {
    Sequence(Vec<String>),
}

impl VirtualNode {
    /// 获取序列路由列表
    ///
    /// 返回按顺序尝试的后端节点名称列表
    pub fn sequence(&self) -> &[String] {
        match self {
            VirtualNode::Sequence(seq) => seq,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{GatewayConfig, Node};
    use std::str::FromStr;

    /// 测试虚拟节点序列路由解析
    #[test]
    fn test_parse_virtual_node_sequence() {
        let toml_str = r#"
[node."qwen3.5-35b-a3b"]
sequence = ["sglang-qwen3.5-35b-a3b", "aliyun"]
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.nodes.contains_key("qwen3.5-35b-a3b"));
        match &config.nodes["qwen3.5-35b-a3b"] {
            Node::Virtual(virtual_node) => {
                assert_eq!(
                    virtual_node.sequence(),
                    &["sglang-qwen3.5-35b-a3b", "aliyun"]
                );
            }
            _ => panic!("Expected Virtual node"),
        }
    }

    /// 测试多个虚拟节点解析
    #[test]
    fn test_parse_multiple_virtual_nodes() {
        let toml_str = r#"
[node."model-a"]
sequence = ["backend-1", "backend-2"]

[node."model-b"]
sequence = ["backend-3", "backend-4"]
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.nodes.contains_key("model-a"));
        assert!(config.nodes.contains_key("model-b"));
    }
}
