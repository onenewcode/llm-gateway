//! 输入节点模块
//!
//! 实现网关的入口节点，负责接收客户端请求并根据模型名称路由到对应的后端

use crate::{Node, RouteError, RoutePayload, RouteResult, health_monitor::HealthMonitor};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// 输入节点结构
///
/// 网关的入口点，维护模型名称到下游节点的映射
#[derive(Clone)]
pub struct InputNode(Arc<Internal>);

struct Internal {
    /// 节点名称
    name: String,
    /// 监听端口
    port: u16,
    /// 模型名称到下游节点的映射
    models: RwLock<HashMap<String, Arc<dyn Node>>>,
    /// 模型别名映射：别名 -> 实际模型名
    alias: HashMap<String, String>,
}

impl Node for InputNode {
    /// 获取节点名称
    fn name(&self) -> &str {
        &self.0.name
    }

    /// 根据请求中的模型名称路由到对应的下游节点
    fn route(&self, payload: &RoutePayload) -> RouteResult {
        match self.0.models.read().unwrap().get(&payload.model) {
            Some(node) => node.route(payload),
            None => Err(RouteError::NoAvailable),
        }
    }

    /// 替换模型名称对应的下游节点引用，建立完整的节点连接
    fn replace_connections(&self, nodes: &HashMap<&str, Arc<dyn Node>>) {
        for node in self.0.models.write().unwrap().values_mut() {
            *node = nodes.get(node.name()).unwrap().clone()
        }
    }

    fn health(&self) -> Option<&Arc<HealthMonitor>> {
        None
    }
}

impl InputNode {
    /// 创建新的输入节点
    pub fn new(
        name: String,
        port: u16,
        models: HashMap<String, Arc<dyn Node>>,
        alias: HashMap<String, String>,
    ) -> Self {
        Self(Arc::new(Internal {
            name,
            port,
            models: RwLock::new(models),
            alias,
        }))
    }

    pub fn port(&self) -> u16 {
        self.0.port
    }

    pub fn models(&self) -> impl IntoIterator<Item = String> {
        self.0
            .models
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Box<[_]>>()
    }

    pub fn get_alias(&self, name: &str) -> Option<String> {
        self.0.alias.get(name).cloned()
    }
}
