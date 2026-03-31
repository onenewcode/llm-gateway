//! 并发控制节点模块
//!
//! 实现并发连接数限制的虚拟节点，仅负责并发控制，将请求转发给单一后继

use crate::{Node, NodeGuard, RouteError, RoutePayload, RouteResult};
use std::any::Any;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// 并发控制守卫，RAII 模式管理并发计数
pub struct ConcurrencyGuard {
    /// 持有对并发计数器的引用，用于在 Drop 时释放计数
    limiter: Arc<AtomicUsize>,
}

impl Drop for ConcurrencyGuard {
    fn drop(&mut self) {
        self.limiter.fetch_sub(1, Ordering::Release);
    }
}

impl NodeGuard for ConcurrencyGuard {
    fn node(&self) -> &dyn Node {
        // ConcurrencyGuard 不直接对应某个节点，这个方法不应该被调用
        unimplemented!("ConcurrencyGuard does not wrap a node")
    }
}

/// 并发控制节点结构
pub struct ConcurrencyNode {
    /// 节点名称
    name: String,
    /// 最大并发数阈值
    max: usize,
    /// 当前并发计数（原子操作）
    current: Arc<AtomicUsize>,
    /// 单一后继节点
    successor: std::sync::RwLock<Option<Arc<dyn Node>>>,
}

impl ConcurrencyNode {
    /// 创建新的并发控制节点
    pub fn new(name: String, max: usize) -> Self {
        Self {
            name,
            max,
            current: Arc::new(AtomicUsize::new(0)),
            successor: std::sync::RwLock::new(None),
        }
    }

    /// 设置后继节点（在构建阶段调用）
    pub fn set_successor(&self, successor: Arc<dyn Node>) {
        *self.successor.write().unwrap() = Some(successor);
    }
}

impl Clone for ConcurrencyNode {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            max: self.max,
            current: self.current.clone(),
            successor: std::sync::RwLock::new(self.successor.read().unwrap().clone()),
        }
    }
}

impl Node for ConcurrencyNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn into_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn route(&self, payload: &RoutePayload) -> RouteResult {
        // 尝试获取许可（原子操作）
        let mut current = self.current.load(Ordering::Relaxed);
        loop {
            if current >= self.max {
                // 已达上限，拒绝
                warn!(
                    "Concurrency limit exceeded for node '{}': current={}, max={}",
                    self.name, current, self.max
                );
                return Err(RouteError::NoAvailable);
            }
            // 尝试增加计数
            match self.current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,                  // 成功
                Err(actual) => current = actual, // 重试
            }
        }

        // 转发给单一后继
        let result = if let Some(ref successor) = *self.successor.read().unwrap() {
            match successor.route(payload) {
                Ok(mut route) => {
                    // 成功：将 Guard 添加到路由
                    route.nodes.push(Box::new(ConcurrencyGuard {
                        limiter: self.current.clone(),
                    }));
                    Ok(route)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(RouteError::NoAvailable)
        };

        // 如果失败，立即释放计数
        if result.is_err() {
            self.current.fetch_sub(1, Ordering::Release);
        }

        result
    }

    fn replace_connections(&self, _nodes: &std::collections::HashMap<&str, Arc<dyn Node>>) {
        // 替换后继节点引用
        if let Some(ref successor) = *self.successor.read().unwrap() {
            *self.successor.write().unwrap() = Some(_nodes.get(successor.name()).unwrap().clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试并发计数正确性
    #[test]
    fn test_concurrent_counting() {
        let node = ConcurrencyNode::new("test".into(), 2);

        // 初始状态应该为 0
        assert_eq!(node.current.load(Ordering::Relaxed), 0);

        // 手动模拟 acquire 操作
        let mut current = node.current.load(Ordering::Relaxed);
        loop {
            match node.current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
        assert_eq!(node.current.load(Ordering::Relaxed), 1);

        // 创建 Guard（不增加计数，只负责释放）
        let guard1 = ConcurrencyGuard {
            limiter: node.current.clone(),
        };

        // 再次手动模拟 acquire
        current = node.current.load(Ordering::Relaxed);
        loop {
            match node.current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
        assert_eq!(node.current.load(Ordering::Relaxed), 2);

        let guard2 = ConcurrencyGuard {
            limiter: node.current.clone(),
        };

        // 释放第一个
        drop(guard1);
        assert_eq!(node.current.load(Ordering::Relaxed), 1);

        // 释放第二个
        drop(guard2);
        assert_eq!(node.current.load(Ordering::Relaxed), 0);
    }
}
