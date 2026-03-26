use crate::{Backend, GatewayError, Node, Route, RouteError, RoutePayload, RouteResult};
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST};
use http::{HeaderName, StatusCode, Uri};
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::{Bytes, Frame};
use hyper::{Request, Response, server::conn::http1, service::service_fn};
use hyper_util::{
    client::legacy::Client,
    rt::{TokioExecutor, TokioIo},
};
use llm_gateway_protocols::streaming::{self, StreamingCollector};
use llm_gateway_protocols::{Protocol, SseCollector, request};
use serde_json::Value;
use std::collections::HashSet;
use std::{
    collections::HashMap,
    fmt::Write,
    sync::{Arc, RwLock},
};
use tokio::net::TcpListener;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, GatewayError>;

pub struct InputNode {
    pub(super) name: Arc<str>,
    pub(super) port: u16,
    pub(super) models: RwLock<HashMap<Arc<str>, Arc<dyn Node>>>,
}

impl Node for InputNode {
    fn name(&self) -> &Arc<str> {
        &self.name
    }

    fn route(&self, payload: &RoutePayload) -> RouteResult {
        let model = payload.body.get("model").and_then(Value::as_str).unwrap();
        match self.models.read().unwrap().get(model) {
            Some(node) => match node.route(payload) {
                Ok(mut route) => {
                    route.nodes.push(self.name.clone());
                    Ok(route)
                }
                Err(e) => Err(e),
            },
            None => Err(RouteError::NoAvailable),
        }
    }

    fn replace_connections(&self, nodes: &HashMap<&str, Arc<dyn Node>>) {
        for node in self.models.write().unwrap().values_mut() {
            *node = nodes
                .get(&**node.name())
                .unwrap_or_else(|| panic!("{}: successor {} not fount", self.name, node.name()))
                .clone()
        }
    }
}

const X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");
const ANTHROPIC_VERSION: HeaderName = HeaderName::from_static("anthropic-version");

impl InputNode {
    /// 运行 HTTP 服务器
    pub async fn run(self: &Arc<Self>) -> Result<(), GatewayError> {
        use std::net::SocketAddr;

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;

        log::info!("Listening on {addr}");

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            log::info!("Accepted connection from {remote_addr}");

            let node = self.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(move |req| {
                    let node = node.clone();
                    async move { node.handle_request(req).await }
                });

                if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                    log::warn!("Error handling connection from {remote_addr}: {e}");
                }
            });
        }
    }

    /// 处理单个 HTTP 请求
    async fn handle_request(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody>, GatewayError> {
        let payload = RoutePayload::new(req).await?;
        match self.route(&payload) {
            Ok(Route { mut nodes, backend }) => {
                // 日志记录路由成功路径
                nodes.reverse();
                let path_str = nodes.join(" -> ");
                log::info!("Routing path: {path_str}");

                if payload.protocol() == backend.protocol {
                    self.forward_to_backend(payload, backend).await
                } else {
                    self.forward_to_foreign(payload, backend).await
                }
            }
            Err(e) => match e {
                RouteError::NoAvailable => {
                    log::warn!("No available backend for this model");
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .body(
                            Full::<Bytes>::from("No available backend for this model")
                                .map_err(|_| GatewayError::NoAvailableBackend)
                                .boxed(),
                        )
                        .unwrap())
                }
            },
        }
    }

    /// 直接转发请求到后端，支持 SSE 流式响应
    async fn forward_to_backend(
        &self,
        payload: RoutePayload,
        backend: Backend,
    ) -> Result<Response<BoxBody>, GatewayError> {
        // 重建 URI
        let uri = format!("{}{}", backend.base_url, payload.parts.uri.path())
            .parse::<Uri>()
            .map_err(|e| GatewayError::BackendRequestFailed(e.to_string()))?;

        // 重建请求
        let mut req_builder = Request::builder()
            .method(payload.parts.method)
            .uri(uri)
            .header(CONTENT_TYPE, "application/json");

        let mut skip_header = HashSet::from([HOST, CONTENT_TYPE, CONTENT_LENGTH]);

        if let Some(api_key) = backend.api_key.as_deref() {
            let mut api_key_added = false;
            if payload.parts.headers.contains_key(X_API_KEY) {
                req_builder = req_builder.header(X_API_KEY, api_key);
                skip_header.insert(X_API_KEY);
                api_key_added = true
            }
            if payload.parts.headers.contains_key(AUTHORIZATION) {
                req_builder = req_builder.header(AUTHORIZATION, api_key);
                skip_header.insert(AUTHORIZATION);
                api_key_added = true
            }
            if !api_key_added {
                match backend.protocol {
                    Protocol::OpenAI => req_builder = req_builder.header(AUTHORIZATION, api_key),
                    Protocol::Anthropic => req_builder = req_builder.header("x-api-key", api_key),
                }
            }
        }

        // 转发所有原始 headers
        for (name, value) in payload.parts.headers {
            if let Some(name) = name
                && !skip_header.contains(&name)
            {
                req_builder = req_builder.header(name, value)
            }
        }

        log::debug!("use headers: {:#?}", req_builder.headers_ref());
        let forward_req: Request<Full<Bytes>> = req_builder
            .body(Full::from(serde_json::to_vec(&payload.body).unwrap()))
            .unwrap();
        let client = Client::builder(TokioExecutor::new()).build_http();

        // 发送请求到后端
        match client.request(forward_req).await {
            Ok(response) => {
                let (parts, body) = response.into_parts();

                // 流式转发后端响应体
                Ok(Response::from_parts(
                    parts,
                    body.map_err(std::io::Error::other)
                        .map_err(GatewayError::IoError)
                        .boxed(),
                ))
            }
            Err(_) => Err(GatewayError::BackendRequestFailed(
                "Failed to connect to backend".into(),
            )),
        }
    }

    /// 直接转发请求到后端，支持 SSE 流式响应
    async fn forward_to_foreign(
        &self,
        payload: RoutePayload,
        backend: Backend,
    ) -> Result<Response<BoxBody>, GatewayError> {
        let protocol = payload.protocol();
        log::info!("forward to foreign: {protocol:?} -> {:?}", backend.protocol);

        // 重建 URI
        let uri = format!("{}{}", backend.base_url, backend.protocol.path())
            .parse::<Uri>()
            .map_err(|e| GatewayError::BackendRequestFailed(e.to_string()))?;

        // 重建请求
        let mut req_builder = Request::builder()
            .method(payload.parts.method)
            .uri(uri)
            .header(CONTENT_TYPE, "application/json");

        let mut skip_header =
            HashSet::from([HOST, CONTENT_TYPE, CONTENT_LENGTH, AUTHORIZATION, X_API_KEY]);

        if matches!(backend.protocol, Protocol::OpenAI) {
            skip_header.insert(ANTHROPIC_VERSION);
        }

        if let Some(api_key) = backend.api_key.as_deref() {
            match backend.protocol {
                Protocol::OpenAI => req_builder = req_builder.header(AUTHORIZATION, api_key),
                Protocol::Anthropic => req_builder = req_builder.header(X_API_KEY, api_key),
            }
        }

        // 转发所有原始 headers
        for (name, value) in payload.parts.headers {
            if let Some(name) = name
                && !skip_header.contains(&name)
            {
                req_builder = req_builder.header(name, value)
            }
        }

        // 协议转换
        let body = match (protocol, backend.protocol) {
            (Protocol::OpenAI, Protocol::Anthropic) => {
                request::openai_to_anthropic(payload.body).unwrap()
            }
            (Protocol::Anthropic, Protocol::OpenAI) => {
                request::anthropic_to_openai(payload.body).unwrap()
            }
            (_, _) => unreachable!(),
        };

        log::debug!("use headers: {:#?}", req_builder.headers_ref());
        let forward_req: Request<Full<Bytes>> = req_builder
            .body(Full::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let client = Client::builder(TokioExecutor::new()).build_http();

        let mut converter: Box<dyn StreamingCollector> = match (protocol, backend.protocol) {
            (Protocol::OpenAI, Protocol::Anthropic) => {
                Box::new(streaming::AnthropicToOpenai::default())
            }
            (Protocol::Anthropic, Protocol::OpenAI) => {
                Box::new(streaming::OpenaiToAnthropic::default())
            }
            (_, _) => unreachable!(),
        };

        // 发送请求到后端
        match client.request(forward_req).await {
            Ok(response) => {
                let (parts, body) = response.into_parts();

                let mut collector = SseCollector::new();
                let mapped = body.map_frame(move |f| {
                    let msgs = collector.collect(f.data_ref().unwrap()).unwrap();
                    let mut ans = String::new();
                    for msg in msgs {
                        log::debug!("in: {msg}");
                        if let Some(out) = converter.process(msg).unwrap() {
                            for line in out {
                                write!(ans, "{line}").unwrap()
                            }
                        }
                    }
                    log::debug!("out: {ans}");
                    Frame::data(Bytes::from(ans))
                });

                // 流式转发后端响应体
                Ok(Response::from_parts(
                    parts,
                    mapped
                        .map_err(std::io::Error::other)
                        .map_err(GatewayError::IoError)
                        .boxed(),
                ))
            }
            Err(_) => Err(GatewayError::BackendRequestFailed(
                "Failed to connect to backend".into(),
            )),
        }
    }
}
