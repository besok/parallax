#[cfg(test)]
mod tests;
use crate::VoidRes;
use crate::error::KernelError;
use crate::servers::ssh::handler::SshHandler;
use crate::servers::{Server, ServerError};
use azure_messaging_servicebus::prelude::TopicClient;
use azure_messaging_servicebus::service_bus::SendMessageOptions;
use base64::{Engine as _, engine::general_purpose};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;

type Topic = String;
type Subscription = String;

#[derive(Debug)]
pub struct AzureTopicClient {
    namespace: String,
    policy: (String, String),
    server_handle: Option<JoinHandle<()>>,
    topic: Topic,
    subscription: Subscription,
    client: Option<TopicClient>,
}

impl Default for AzureTopicClient {
    fn default() -> Self {
        AzureTopicClient::new(
            "localhost",
            ("RootManageSharedAccessKey", "SAS_KEY_VALUE"),
            "test-topic",
            "test-sub",
        )
    }
}

impl AzureTopicClient {
    pub fn new(
        namespace: impl Into<String>,
        policy: (impl Into<String>, impl Into<String>),
        topic: impl Into<Topic>,
        subscription: impl Into<Subscription>,
    ) -> Self {
        AzureTopicClient {
            server_handle: None,
            namespace: namespace.into(),
            policy: (policy.0.into(), policy.1.into()),
            topic: topic.into(),
            subscription: subscription.into(),
            client: None,
        }
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.topic, self.subscription)
    }
}

impl Server<AzureMessage> for AzureTopicClient {
    fn start(&mut self) -> VoidRes {
        log::info!(
            "Starting Azure Client  {}  - for now it is empty",
            self.id()
        );
        // let namespace = self.namespace.clone();
        // let sub = self.subscription.clone();
        // let (name, key) = self.policy.clone();
        // let topic = self.topic.clone();

        // let request_client = ::reqwest::ClientBuilder::new()
        //     .pool_max_idle_per_host(0)
        //     .build()
        //     .map_err(|e| {
        //         KernelError::ServerError(ServerError::StartError(e.to_string(), self.id()))
        //     })?;
        //
        // self.client = Some(
        //     TopicClient::new(Arc::new(request_client), namespace, topic, name, key).map_err(
        //         |e| KernelError::ServerError(ServerError::StartError(e.to_string(), self.id())),
        //     )?,
        // );

        // let client = self.client.clone().unwrap();
        //
        // self.server_handle = Some(tokio::spawn(async move {
        //     let sub_receiver = client.subscription_receiver(sub.as_str());
        //     loop {
        //         match sub_receiver.receive_and_delete_message().await {
        //             Ok(message) => {
        //                 log::info!("Received message: {}", message);
        //             }
        //             Err(e) => {
        //                 log::error!("Error receiving message: {:?}", e);
        //                 sleep(Duration::from_millis(500)).await;
        //             }
        //         }
        //     }
        // }));
        Ok(())
    }

    fn stop(&mut self) -> VoidRes {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            log::info!("Azure Client stopped");
        }
        Ok(())
    }

    fn process(&mut self, message: AzureMessage) -> VoidRes {
        match message {
            AzureMessage::Start => {
                log::info!("Azure Client started");
                self.start()
            }
            AzureMessage::Stop => {
                log::info!("Azure Client stopped");
                self.stop()
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

use std::process::Command;

fn send_azure_message(topic: &str, message: &[u8]) -> Result<(), std::io::Error> {
    // Convert binary message to base64

    let b64 = general_purpose::STANDARD.encode(message);

    let status = Command::new("python")
        .arg("src/servers/azure/sb-emulator/send_message.py")
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
