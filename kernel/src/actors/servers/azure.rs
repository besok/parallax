#[cfg(test)]
mod tests;
use crate::VoidRes;
use crate::actors::servers::ServerError;
use crate::error::KernelError;
use base64::{Engine as _, engine::general_purpose};
use tokio::task::JoinHandle;

type Topic = String;
type Subscription = String;

#[derive(Debug)]
pub struct AzureTopicListener<H: ServiceBusMsgHandler = EchoServiceBusMsgHandler> {
    conn: String,
    topic: Topic,
    subscription: Subscription,
    server_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<Sender<()>>,
    handler: H,
}

pub trait ServiceBusMsgHandler {
    fn process(&mut self, message: &ServiceBusReceivedMessage) -> VoidRes;
}

#[derive(Debug, Clone, Default)]
pub struct EchoServiceBusMsgHandler;

impl ServiceBusMsgHandler for EchoServiceBusMsgHandler {
    fn process(&mut self, message: &ServiceBusReceivedMessage) -> VoidRes {
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
            H::default(),
        )
    }
}

impl<H: ServiceBusMsgHandler> AzureTopicListener<H> {
    pub fn new(conn: &str, topic: &str, subscription: &str, handler: H) -> Self {
        AzureTopicListener {
            conn: conn.to_string(),
            topic: topic.to_string(),
            subscription: subscription.to_string(),
            server_handle: None,
            shutdown_tx: None,
            handler,
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
    fn start(&mut self) -> VoidRes {
        log::info!("Starting Azure Client  {}", self.id());

        let conn = self.conn.clone();
        let topic = self.topic.clone();
        let subscription = self.subscription.clone();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);
        let mut h = self.handler.clone();
        self.server_handle = Some(tokio::spawn(async move {
            log::info!("Connecting to Azure Service Bus at {}", conn);
            if let Err(e) = async {
                let mut client = ServiceBusClient::new_from_connection_string(
                    &conn,
                    ServiceBusClientOptions::default(),
                )
                    .await?;
                let mut receiver = client
                    .create_receiver_for_subscription(
                        &topic,
                        &subscription,
                        ServiceBusReceiverOptions::default(),
                    )
                    .await?;
                log::info!(
                    "Receiver created for topic: {}, subscription: {}",
                    topic,
                    subscription
                );

                loop {
                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            log::info!("Shutdown signal received, stopping Azure client");
                            break;
                        }

                        message_result = receiver.receive_message_with_max_wait_time(Some(Duration::from_millis(200))) => {
                            match message_result {
                                Ok(Some(msg)) => {
                                    log::info!("Received message: {:?}", msg);
                                    if let Err(e) = h.process(&msg){
                                        log::error!("Error processing message: {:?}", e);
                                        
                                    }
                                    if let Err(e) = receiver.complete_message(&msg).await {
                                        log::error!("Failed to complete message: {}", e);
                                    }
                                }
                                Ok(None) => {
                                    log::debug!("No message received, continuing to wait");
                                }
                                Err(e) => {
                                    log::error!("Error receiving message: {:?}", e);
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                }

                            }
                        }
                    }
                }

                receiver.dispose().await?;
                client.dispose().await?;

                Ok::<(), KernelError>(())
            }
                .await
            {
                log::error!("Error connecting to Azure Service Bus {e:?}")
            };
        }));

        Ok(())
    }

    fn stop(&mut self) -> VoidRes {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.try_send(());

            if let Some(handle) = self.server_handle.take() {
                tokio::spawn(async move {
                    match tokio::time::timeout(Duration::from_secs(60), handle).await {
                        Ok(_) => log::info!("Azure Topic Listener shut down successfully"),
                        Err(_) => log::warn!("Azure Topic Listener shutdown timed out"),
                    }
                });
            }
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

use crate::actors::Actor;
use azservicebus::core::BasicRetryPolicy;
use azservicebus::{
    ServiceBusClient, ServiceBusClientOptions, ServiceBusReceivedMessage, ServiceBusReceiverOptions,
};
use std::process::Command;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task;

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
