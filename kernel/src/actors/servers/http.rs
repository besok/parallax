#[cfg(test)]
mod tests;

use crate::VoidRes;
use crate::actors::Actor;
use crate::actors::servers::ServerError;
use axum::routing::get;
use axum::{Json, Router};
use std::net::SocketAddr;
use std::time::Duration;
use bevy::prelude::Component;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct BaseHttpServer {
    id: String,
    host: String,
    port: u16,
    router: Router,
    server_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<Sender<()>>,
}

impl Default for BaseHttpServer {
    fn default() -> Self {
        BaseHttpServer::new("http_server", "127.0.0.1", 8080, None)
    }
}

pub fn default_router() -> Router {
    Router::new().route(
        "/health",
        get(|| async { Json(serde_json::json!({ "status": "up" })) }),
    )
}

impl BaseHttpServer {
    pub fn new(
        id: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        router: Option<Router>,
    ) -> Self {
        BaseHttpServer {
            id: id.into(),
            host: host.into(),
            port,
            router: router.unwrap_or_else(default_router),
            server_handle: None,
            shutdown_tx: None,
        }
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }
}

impl Actor<HttpMessage> for BaseHttpServer {
    async fn start(&mut self) -> VoidRes {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let addr = format!("{}:{}", self.host, self.port)
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::StartError(e.to_string(), self.id.clone()))?;

        log::info!("Starting HTTP server on {}", addr);

        let graceful = axum::Server::bind(&addr)
            .serve(self.router.clone().into_make_service())
            .with_graceful_shutdown(async move {
                shutdown_rx.recv().await;
                log::info!("Shutdown signal received, stopping HTTP server");
            });

        let handle = tokio::spawn(async move {
            match graceful.await {
                Ok(_) => log::info!("HTTP server stopped gracefully"),
                Err(e) => log::error!("HTTP server error: {}", e),
            }
        });

        self.server_handle = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);

        Ok(())
    }

    async fn stop(&mut self) -> VoidRes {
        log::info!("Stopping HTTP server");
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());

            if let Some(handle) = self.server_handle.take() {
                tokio::spawn(async move {
                    match tokio::time::timeout(Duration::from_secs(60), handle).await {
                        Ok(_) => log::info!("HTTP server shut down successfully"),
                        Err(_) => log::warn!("HTTP server shutdown timed out"),
                    }
                });
            }
        }

        Ok(())
    }

    async fn process(&mut self, message: HttpMessage) -> VoidRes {
        log::info!("Processing message: {:?}", message);
        match message {
            HttpMessage::Start => {
                log::info!("Http Server [id={}] received start message", self.id);
                self.start().await?;
            }
            HttpMessage::Stop => {
                log::info!("Http Server [id={}] received stop message", self.id);
                self.stop().await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum HttpMessage {
    Start,
    Stop,
}
