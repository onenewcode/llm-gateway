use crate::{Backend, Node, Route, RoutePayload, RouteResult};
use llm_gateway_config::BaseUrl;
use std::{collections::HashMap, sync::Arc};

pub(crate) struct BackendNode {
    pub(super) name: Arc<str>,
    pub(super) base_url: BaseUrl,
    pub(super) api_key: Option<String>,
}

impl Node for BackendNode {
    fn name(&self) -> &Arc<str> {
        &self.name
    }

    fn route(&self, payload: &RoutePayload) -> RouteResult {
        let protocol = payload.protocol();
        Ok(match self.base_url.get(protocol.name()) {
            Some(base_url) => Route {
                nodes: vec![self.name.clone()],
                backend: Backend {
                    protocol: protocol,
                    base_url: base_url.into(),
                    api_key: self.api_key.clone(),
                },
            },
            None => todo!(),
        })
    }

    fn replace_connections(&self, _: &HashMap<&str, Arc<dyn Node>>) {}
}
