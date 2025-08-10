use actix::{
    Actor, ActorFutureExt, AsyncContext, Context, Handler, Message, Recipient, WrapFuture,
};
use actix_web::dev::ServerHandle;
use actix_web::web::ServiceConfig;
use actix_web::{App, HttpResponse, HttpServer, Result as ActixResult, web};
use actor::{ActorError, ActorResult, ActorResultVoid, ActorServiceMessage};
use std::sync::Arc;
use tokio::sync::oneshot;

pub struct BaseHttpServer {
    key: String,
    host: String,
    port: u16,
    router_config: Option<Arc<dyn Fn(&mut ServiceConfig) + Send + Sync>>,
    server_handle: Option<ServerHandle>,
}

fn default_config() -> Arc<dyn Fn(&mut ServiceConfig) + Send + Sync> {
    Arc::new(|cfg| {
        cfg.route("/ping", web::get().to(ping_handler))
            .route("/health", web::get().to(health_handler));
    })
}
impl Default for BaseHttpServer {
    fn default() -> Self {
        Self::new("http_server", "127.0.0.1", 8080, None)
    }
}

impl BaseHttpServer {
    pub fn new(
        key: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        mb_router_config: Option<Arc<dyn Fn(&mut ServiceConfig) + Send + Sync>>,
    ) -> Self {
        Self {
            key: key.into(),
            host: host.into(),
            port,
            router_config: Some(mb_router_config.unwrap_or_else(default_config)),
            server_handle: None,
        }
    }

    fn prepare_start(&mut self) -> ActorResultVoid {
        if self.server_handle.is_some() {
            Err(ActorError::StartupError(
                "Server is already running".to_string(),
            ))
        } else {
            let bind_addr = format!("{}:{}", self.host, self.port);
            log::info!("[{}] Starting HTTP server on {}", self.key, bind_addr);

            let app_config = self
                .router_config
                .take()
                .ok_or(ActorError::StartupError("unexpected!".to_string()))?;

            let server = HttpServer::new(move || {
                App::new()
                    .configure(|ctx| app_config(ctx))
                    .wrap(actix_web::middleware::Logger::default())
                    .wrap(actix_web::middleware::Compress::default())
            })
            .bind(&bind_addr)
            .map_err(|e| {
                ActorError::StartupError(format!("Failed to bind to {}: {}", bind_addr, e))
            })?
            .workers(4)
            .keep_alive(std::time::Duration::from_secs(75))
            .shutdown_timeout(30);

            let server_runner = server.run();
            let server_handle = server_runner.handle();
            self.server_handle = Some(server_handle);

            Ok(())
        }
    }

    fn stop(&mut self) -> ActorResultVoid {
        log::info!("[{}] Stopping HTTP server", self.key);
        if let Some(handle) = self.server_handle.take() {
            _ = handle.stop(true);
            log::info!("[{}] HTTP server is stopping", self.key);
        }
        Ok(())
    }
}

async fn ping_handler() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "pong",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn health_handler() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "up",
        "service": "http-actor",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

impl Actor for BaseHttpServer {
    type Context = Context<Self>;
}

impl Handler<ActorServiceMessage> for BaseHttpServer {
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ActorServiceMessage::Start => {
                self.prepare_start()?;
                log::info!("[{}] HTTP server started successfully", self.key);
            }
            ActorServiceMessage::Stop => {
                self.stop();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::prelude::*;
    use utils::logger_on;

    #[actix::test]
    async fn test_server_creation() {
        let server = BaseHttpServer::default();
        assert_eq!(server.key, "http_server");
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 8080);
    }

    #[actix::test]
    async fn test_custom_server() {
        let server = BaseHttpServer::new("test_server", "0.0.0.0", 3000, None);
        assert_eq!(server.key, "test_server");
        assert_eq!(server.host, "0.0.0.0");
        assert_eq!(server.port, 3000);
    }

    #[actix::test]
    async fn test_server_start_stop() {
        logger_on();
        let server = BaseHttpServer::default();
        let addr = server.start();

        let start_result = addr.send(ActorServiceMessage::Start).await;
        assert!(start_result.is_ok());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let stop_result = addr.send(ActorServiceMessage::Stop).await;
        assert!(stop_result.is_ok());
    }
}
