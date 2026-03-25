use crate::{Node, RouteError, RoutePayload, RouteResult};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub(crate) struct SequenceNode {
    pub(super) name: Arc<str>,
    pub(super) successors: RwLock<Vec<Arc<dyn Node>>>,
}

impl Node for SequenceNode {
    fn name(&self) -> &Arc<str> {
        &self.name
    }

    fn route(&self, payload: &RoutePayload) -> RouteResult {
        for successor in &*self.successors.read().unwrap() {
            match successor.route(payload) {
                Ok(mut route) => {
                    route.nodes.push(self.name.clone());
                    return Ok(route);
                }
                Err(e) => match e {
                    RouteError::NoAvailable => {}
                },
            }
        }
        Err(RouteError::NoAvailable)
    }

    fn replace_connections(&self, nodes: &HashMap<&str, Arc<dyn Node>>) {
        for node in &mut *self.successors.write().unwrap() {
            *node = nodes.get(&**node.name()).unwrap().clone()
        }
    }
}
