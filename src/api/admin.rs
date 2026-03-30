//! 管理 API 服务

use hyper::server::conn::http1;
use hyper::service::service_fn;
use llm_gateway_statistics::StatsStoreManager;
use log::{info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::api::handlers::handle_request;
use crate::api::middleware::AuthMiddleware;

/// 管理 API 服务
pub struct AdminServer {
    port: u16,
    auth_token: Option<String>,
    store: Arc<StatsStoreManager>,
}

impl AdminServer {
    pub fn new(port: u16, auth_token: Option<String>, store: Arc<StatsStoreManager>) -> Self {
        Self {
            port,
            auth_token,
            store,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;

        let actual_addr = listener.local_addr()?;
        info!(
            "Admin API listening on http://{actual_addr}/v1",
            actual_addr = actual_addr
        );

        let auth = Arc::new(AuthMiddleware::new(self.auth_token.clone()));
        let store = Arc::clone(&self.store);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let auth = Arc::clone(&auth);
            let store = Arc::clone(&store);

            tokio::task::spawn(async move {
                let service = service_fn(move |req| {
                    let auth = Arc::clone(&auth);
                    let store = Arc::clone(&store);
                    handle_request(req, auth, store, peer_addr)
                });

                if let Err(e) = http1::Builder::new()
                    .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                    .await
                {
                    warn!("Connection error: {e}", e = e);
                }
            });
        }
    }
}
