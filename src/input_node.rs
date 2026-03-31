//! 输入节点模块
//!
//! 实现网关的入口节点，负责接收客户端请求并根据模型名称路由到对应的后端

use crate::{Node, RouteError, RoutePayload, RouteResult, SimpleGuard};
use serde_json::Value;
use std::{
    any::Any,
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// 输入节点结构
///
/// 网关的入口点，维护模型名称到下游节点的映射
pub struct InputNode {
    /// 节点名称
    pub(super) name: String,
    /// 监听端口
    pub(super) port: u16,
    /// 模型名称到下游节点的映射
    pub(super) models: RwLock<HashMap<String, Arc<dyn Node>>>,
    /// 模型别名映射：别名 -> 实际模型名
    pub(super) alias: HashMap<String, String>,
}

impl InputNode {
    /// 获取节点名称
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Node for InputNode {
    /// 获取节点名称
    fn name(&self) -> &str {
        &self.name
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    /// 根据请求中的模型名称路由到对应的下游节点
    fn route(&self, payload: &RoutePayload) -> RouteResult {
        let model = payload.body.get("model").and_then(Value::as_str).unwrap();

        match self.models.read().unwrap().get(model) {
            Some(node) => match node.route(payload) {
                Ok(mut route) => {
                    route.nodes.push(Box::new(SimpleGuard::new(node.clone())));
                    Ok(route)
                }
                Err(e) => Err(e),
            },
            None => Err(RouteError::NoAvailable),
        }
    }

    /// 替换模型名称对应的下游节点引用，建立完整的节点连接
    fn replace_connections(&self, nodes: &HashMap<&str, Arc<dyn Node>>) {
        for node in self.models.write().unwrap().values_mut() {
            *node = nodes
                .get(node.name())
                .unwrap_or_else(|| panic!("{}: successor {} not found", self.name, node.name()))
                .clone()
        }
    }
}
