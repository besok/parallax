#[cfg(test)]
mod tests;

use crate::VoidRes;
use crate::actors::servers::ServerError;
use crate::error::KernelError;
use base64::{Engine as _, engine::general_purpose};
use std::io;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;
type Topic = String;
type Subscription = String;
use azure_messaging_servicebus::prelude::*;

pub struct AzureTopicListener<H: ServiceBusMsgHandler = EchoServiceBusMsgHandler> {
    conn: String,
    topic: Topic,
    subscription: Subscription,
    port: u16,
    handler: H,
    http_handle: Option<ActorHandle<HttpMessage>>,
    py_handle: Option<JoinHandle<Result<(), io::Error>>>,
}

pub trait ServiceBusMsgHandler {
    fn process(&mut self, message: &str) -> VoidRes;
}

#[derive(Debug, Clone, Default)]
pub struct EchoServiceBusMsgHandler;

impl ServiceBusMsgHandler for EchoServiceBusMsgHandler {
    fn process(&mut self, message: &str) -> VoidRes {
        log::info!("Echoing message: {:?}", message);
        Ok(())
    }
}

impl<H: ServiceBusMsgHandler + Default> Default for AzureTopicListener<H> {
    fn default() -> Self {
        AzureTopicListener::new(
            "Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;",
            "test-topic",
            "test-sub",
            10000,
            H::default(),
        )
    }
}

impl<H: ServiceBusMsgHandler> AzureTopicListener<H> {
    pub fn new(conn: &str, topic: &str, subscription: &str, port: u16, handler: H) -> Self {
        AzureTopicListener {
            conn: conn.to_string(),
            topic: topic.to_string(),
            subscription: subscription.to_string(),
            port,
            handler,
            http_handle: None,
            py_handle: None,
        }
    }
    pub fn id(&self) -> String {
        format!("{}-{}", self.topic, self.subscription)
    }
}

impl<H> Actor<AzureMessage> for AzureTopicListener<H>
where
    H: ServiceBusMsgHandler + Clone + Send + Sync + 'static,
{
    fn id(&self) -> String {
        format!("{}_{}", self.topic, self.subscription)
    }

    // we have to use the python sdk since the Rust SDK does not support the emulator
    async fn start(&mut self) -> VoidRes {
        log::info!("Starting Azure Client  {}", self.id());

        let conn = self.conn.clone();
        let topic = self.topic.clone();
        let subscription = self.subscription.clone();
        let handler = self.handler.clone();
        let http_server = BaseHttpServer::new(
            self.id(),
            "0.0.0.0",
            self.port,
            Some(Router::new().route(
                "/ret",
                post(|body: axum::body::Bytes| async move {
                    log::info!("Received message: {:?}", String::from_utf8_lossy(&body));
                    if let Err(err) = handler.clone().process(&String::from_utf8_lossy(&body)) {
                        log::error!("Error processing message: {:?}", err);
                        return axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                    }

                    axum::http::StatusCode::OK
                }),
            )),
        );

        self.http_handle = Some(spawn_actor_with(http_server, None).await?);

        self.py_handle = Some(start_py_server(
            conn,
            subscription,
            topic.clone(),
            format!("http://localhost:{}/ret", self.port),
        ));

        Ok(())
    }

    async fn stop(&mut self) -> VoidRes {
        log::info!("Stopping Azure Client {}", self.id());
        if let Some(h) = self.http_handle.take() {
            if let Some(py_handle) = self.py_handle.take() {
                log::info!("Stopping Python process for Azure Client {}", self.id());
                py_handle.abort();
            }
            h.send_sync(HttpMessage::Stop)
        } else {
            Ok(())
        }
    }

    async fn process(&mut self, message: AzureMessage) -> VoidRes {
        match message {
            AzureMessage::Start => {
                log::info!("Azure Client started");
                self.start().await
            }
            AzureMessage::Stop => {
                log::info!("Azure Client stopped");
                self.stop().await
            }
            AzureMessage::SendMessage(msg) => {
                log::info!(
                    "Sending message: {}",
                    String::from_utf8(msg.clone()).map_err(|e| {
                        KernelError::ServerError(ServerError::RuntimeError(e.to_string()))
                    })?
                );
                send_azure_message(&self.topic, msg.as_slice()).map_err(|e| {
                    KernelError::ServerError(ServerError::RuntimeError(e.to_string()))
                })?;
                Ok(())
            }
        }
    }
}

use crate::actors::{Actor, ActorHandle, spawn_actor_with};

use crate::actors::servers::http::{BaseHttpServer, HttpMessage};
use axum::Router;
use axum::routing::post;
use std::process::Command;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task;

fn start_py_server(
    conn: String,
    sub: String,
    topic: String,
    ret_url: String,
) -> JoinHandle<Result<(), std::io::Error>> {
    let handle = tokio::spawn(async move {
        let mut child = tokio::process::Command::new("python")
            .arg("src/actors/servers/azure/sb-emulator/client_test.py")
            .arg("--topic")
            .arg(topic)
            .arg("--sub")
            .arg(sub)
            .arg("--conn")
            .arg(conn)
            .arg("--ret")
            .arg(ret_url)
            .env("PYTHONUNBUFFERED", "1")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                log::error!("Failed to start Python script: {}", e);
                e
            })?;

        // Create tasks to handle stdout and stderr
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Spawn task to handle stdout
        let stdout_task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                log::info!("Python stdout: {}", line);
            }
        });

        // Spawn task to handle stderr
        let stderr_task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                log::error!("Python stderr: {}", line);
            }
        });

        // Wait for the process to complete
        if let Err(e) = child.wait().await {
            log::error!("Error waiting for Python process: {}", e);
        }

        // Abort stream handling tasks when the process is done
        stdout_task.abort();
        stderr_task.abort();

        Ok::<_, std::io::Error>(())
    });

    handle
}
fn send_azure_message(topic: &str, message: &[u8]) -> Result<(), std::io::Error> {
    // Convert binary message to base64

    let b64 = general_purpose::STANDARD.encode(message);

    let status = Command::new("python")
        .arg("src/actors/servers/azure/sb-emulator/send_message.py")
        .arg("--topic")
        .arg(topic)
        .arg("--base64")
        .arg(b64)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Python script failed with status: {}", status),
        ))
    }
}

pub enum AzureMessage {
    Start,
    Stop,
    SendMessage(Vec<u8>),
}
