mod amqp;
#[cfg(test)]
mod tests;

use crate::error::KernelError;
use crate::servers::azure::amqp::handle_amqp_connection;
use crate::servers::http::{BaseHttpServer, HttpMessage};
use crate::servers::{Server, ServerError, ServerHandle, ServerId, spawn_server};
use crate::{Res, VoidRes};
use axum::{
    Router,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use lapin::{
    BasicProperties, Connection, ConnectionProperties, Consumer,
    message::DeliveryResult,
    options::{BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::spawn_local;
use tokio::time::sleep;

type Topic = String;
type Queue = String;
type Subscription = String;
type QueuesState = Arc<Mutex<HashMap<String, Vec<ServiceBusMessage>>>>;
type TopicsState = Arc<Mutex<HashMap<String, HashMap<String, Vec<ServiceBusMessage>>>>>;

#[derive(Clone)]
struct BusState {
    queues: QueuesState,
    topics: TopicsState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBusMessage {
    pub message_id: Option<String>,
    pub body: Vec<u8>,
    pub properties: HashMap<String, String>,
    pub content_type: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug)]
pub enum AzureServiceBusCommand {
    Stop,
    Start,
    AddQueue(Queue),
    AddTopic(Topic, Vec<Subscription>),
    PublishMessage(Topic, ServiceBusMessage),
    ClearQueues,
    ClearTopics,
}

impl BusState {
    fn new() -> Self {
        BusState {
            queues: Arc::new(Mutex::new(HashMap::new())),
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

// Mock Azure Service Bus server
pub struct AzureServiceBusServer {
    id: ServerId,
    http_port: u16,
    amqp_port: u16,
    hostname: String,
    state: BusState,
    http_handle: Option<ServerHandle<HttpMessage>>,
    amqp_handle: Option<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl Default for AzureServiceBusServer {
    fn default() -> Self {
        Self::new("azure-service-bus-mock", "127.0.0.1", 5672, 5671)
    }
}

impl AzureServiceBusServer {
    pub fn new(
        id: impl Into<String>,
        hostname: impl Into<String>,
        http_port: u16,
        amqp_port: u16,
    ) -> Self {
        AzureServiceBusServer {
            id: id.into(),
            http_port,
            amqp_port,
            hostname: hostname.into(),
            state: BusState::new(),
            http_handle: None,
            amqp_handle: None,
            shutdown_tx: None,
        }
    }

    pub fn connection_string(&self) -> String {
        format!(
            "Endpoint=sb://{}:{}/;SharedAccessKeyName=mock;SharedAccessKey=mock_key;SharedAccessKeyName=RootManageSharedAccessKey",
            self.hostname, self.amqp_port
        )
    }

    fn add_queue(&mut self, queue_name: String) -> VoidRes {
        let mut queues = self.state.queues.lock()?;
        queues.entry(queue_name).or_insert_with(Vec::new);
        Ok(())
    }

    fn add_topic(&mut self, topic_name: String, subscriptions: Vec<String>) -> VoidRes {
        let mut topics = self.state.topics.lock()?;
        let topic_map = topics.entry(topic_name).or_insert_with(HashMap::new);

        for subscription in subscriptions {
            topic_map.entry(subscription).or_insert_with(Vec::new);
        }
        Ok(())
    }

    fn clear_queues(&mut self) -> VoidRes {
        let mut queues = self.state.queues.lock()?;
        queues.clear();
        Ok(())
    }

    fn clear_topics(&mut self) -> VoidRes {
        let mut topics = self.state.topics.lock()?;
        topics.clear();
        Ok(())
    }

    fn start_amqp_server(&mut self, mut shutdown_rx: mpsc::Receiver<()>) -> VoidRes {
        let state = self.state.clone();
        let host = self.hostname.clone();
        let port = self.amqp_port;
        let id = self.id.clone();
        let addr = format!("amqp://guest:guest@{}:{}", host, self.amqp_port);

        log::info!("Starting AMQP server on {}", addr);

        self.amqp_handle = Some(tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("Failed to bind AMQP server: {:?}", e);
                    return;
                }
            };

            log::info!("AMQP server listening on {}:{}", host, port);

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        log::info!("Shutdown signal received, stopping AMQP server");
                        return ;
                    }
                    // Handle incoming connections
                    conn_result = listener.accept() => {
                        match conn_result {
                            Ok((stream, addr)) => {
                                log::info!("New AMQP connection from: {}", addr);

                                let state_clone = state.clone();

                                spawn_local(async move {
                                    if let Err(e) = handle_amqp_connection(stream, state_clone).await {
                                        log::error!("Error handling AMQP connection: {:?}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::error!("Error accepting AMQP connection: {}", e);
                                 sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
            }
        }));
        Ok(())
    }
}

// HTTP server route handlers
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "up",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn authorize() -> impl IntoResponse {
    Json(serde_json::json!({
        "token_type": "Bearer",
        "expires_in": 3600,
        "access_token": "mock_access_token_for_azure_service_bus"
    }))
}

async fn get_connection_string(State(state): State<BusState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "connection_string": format!(
            "Endpoint=sb://localhost:5671/;SharedAccessKeyName=mock;SharedAccessKey=mock_key;SharedAccessKeyName=RootManageSharedAccessKey"
        )
    }))
}

impl Server<AzureServiceBusCommand> for AzureServiceBusServer {
    fn start(&mut self) -> VoidRes {
        let app_state = self.state.clone();

        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        self.shutdown_tx = Some(shutdown_tx.clone());

        let app = Router::new()
            .route("/health", get(health))
            .route("/token", post(authorize))
            .route("/.well-known/oauth-authorization-server", get(authorize))
            .route("/connection-string", get(get_connection_string))
            .with_state(app_state.clone());

        let http_handle = spawn_server(
            BaseHttpServer::new(
                format!("{}-Http-base-server", self.id),
                &self.hostname,
                self.http_port,
                Some(app),
            ),
            None,
        )?;

        self.http_handle = Some(http_handle);
        self.start_amqp_server(shutdown_rx)?;

        log::info!(
            "Azure Service Bus mock server started with AMQP on port {} and HTTP on port {}",
            self.amqp_port,
            self.http_port
        );

        Ok(())
    }

    fn stop(&mut self) -> VoidRes {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());
            log::info!("Shutdown signal sent to Azure Service Bus mock server");
        }

        if let Some(h) = &self.http_handle {
            h.send_sync(HttpMessage::Stop)?;
        }

        if let Some(handle) = self.amqp_handle.take() {
            handle.abort();
        }

        Ok(())
    }

    fn process(&mut self, message: AzureServiceBusCommand) -> VoidRes {
        match message {
            AzureServiceBusCommand::Stop => self.stop()?,

            AzureServiceBusCommand::AddQueue(queue_name) => {
                self.add_queue(queue_name)?;
            }
            AzureServiceBusCommand::AddTopic(topic_name, subscriptions) => {
                self.add_topic(topic_name, subscriptions)?;
            }
            AzureServiceBusCommand::ClearQueues => {
                self.clear_queues()?;
            }
            AzureServiceBusCommand::ClearTopics => {
                self.clear_topics()?;
            }
            AzureServiceBusCommand::Start => {
                self.start()?;
            }
            AzureServiceBusCommand::PublishMessage(topic_name, message) => {
                let mut topics = self.state.topics.lock()?;
                if let Some(subscriptions) = topics.get_mut(&topic_name) {
                    for subscription in subscriptions.values_mut() {
                        subscription.push(message.clone());
                    }
                }
            }
        }
        Ok(())
    }
}
