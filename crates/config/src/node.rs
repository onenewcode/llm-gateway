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
    /// 输入节点
    Input(InputNode),
    /// 虚拟节点
    Virtual(VirtualNode),
    /// 后端节点
    Backend(BackendNode),
}

/// 虚拟节点类型
///
/// 虚拟节点用于定义路由策略，目前支持：
/// - [`Sequence`](VirtualNode::Sequence) - 顺序尝试，按列表顺序尝试后端
/// - [`Concurrency`](VirtualNode::Concurrency) - 并发控制，限制并发请求数
#[derive(Clone, Debug)]
pub enum VirtualNode {
    /// 顺序尝试路由
    Sequence(Vec<String>),
    /// 并发控制节点
    Concurrency {
        /// 最大并发数
        max: usize,
        /// 单一后继节点
        successor: String,
    },
}

impl VirtualNode {
    /// 获取序列路由列表
    ///
    /// 返回按顺序尝试的后端节点名称列表
    /// 对于 Concurrency 节点，返回包含单一后继的切片
    pub fn sequence(&self) -> &[String] {
        match self {
            VirtualNode::Sequence(seq) => seq,
            VirtualNode::Concurrency { successor, .. } => {
                // 使用 std::slice::from_ref 返回单元素切片
                std::slice::from_ref(successor)
            }
        }
    }

    /// 获取并发控制配置（仅当为 Concurrency 变体时）
    pub fn concurrency(&self) -> Option<usize> {
        match self {
            VirtualNode::Concurrency { max, .. } => Some(*max),
            _ => None,
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

    /// 测试并发控制虚拟节点解析
    #[test]
    fn test_parse_concurrency_node() {
        let toml_str = r#"
[node."qwen3.5-35b-a3b"]
concurrency = { max = 100, successor = "backend-1" }
"#;
        let result = GatewayConfig::from_str(toml_str);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert!(config.nodes.contains_key("qwen3.5-35b-a3b"));
        match &config.nodes["qwen3.5-35b-a3b"] {
            Node::Virtual(virtual_node) => {
                assert_eq!(virtual_node.concurrency(), Some(100));
                assert_eq!(virtual_node.sequence(), &["backend-1"]);
            }
            _ => panic!("Expected Virtual node"),
        }
    }
}
