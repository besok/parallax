#[cfg(test)]
mod tests;
use crate::VoidRes;
use crate::error::KernelError;
use crate::servers::ssh::handler::SshHandler;
use crate::servers::{Server, ServerError};
use azure_messaging_servicebus::prelude::TopicClient;
use azure_messaging_servicebus::service_bus::SendMessageOptions;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;

type Topic = String;
type Subscription = String;

#[derive(Debug)]
pub struct AzureNativeTopicClient {
    namespace: String,
    policy: (String, String),
    server_handle: Option<JoinHandle<()>>,
    topic: Topic,
    subscription: Subscription,
    client: Option<TopicClient>,
}

impl Default for AzureNativeTopicClient {
    fn default() -> Self {
        AzureNativeTopicClient::new(
            "localhost",
            ("RootManageSharedAccessKey", "SAS_KEY_VALUE"),
            "test-topic",
            "test-sub",
        )
    }
}

impl AzureNativeTopicClient {
    pub fn new(
        namespace: impl Into<String>,
        policy: (impl Into<String>, impl Into<String>),
        topic: impl Into<Topic>,
        subscription: impl Into<Subscription>,
    ) -> Self {
        AzureNativeTopicClient {
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

impl Server<AzureMessage> for AzureNativeTopicClient {
    fn start(&mut self) -> VoidRes {
        log::info!("Starting Azure Client  {} ", self.id());
        let namespace = self.namespace.clone();
        let sub = self.subscription.clone();
        let (name, key) = self.policy.clone();
        let topic = self.topic.clone();

        let request_client = ::reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|e| {
                KernelError::ServerError(ServerError::StartError(e.to_string(), self.id()))
            })?;

        self.client = Some(
            TopicClient::new(Arc::new(request_client), namespace, topic, name, key).map_err(
                |e| KernelError::ServerError(ServerError::StartError(e.to_string(), self.id())),
            )?,
        );
        let client = self.client.clone().unwrap();

        self.server_handle = Some(tokio::spawn(async move {
            let sub_receiver = client.subscription_receiver(sub.as_str());
            loop {
                match sub_receiver.receive_and_delete_message().await {
                    Ok(message) => {
                        log::info!("Received message: {}", message);
                    }
                    Err(e) => {
                        log::error!("Error receiving message: {:?}", e);
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }));
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
            AzureMessage::SendMessage(msg, opts) => {
                log::info!("Sending message: {}", msg);
                if let Some(client) = self.client.as_ref() {
                    let sender = client.topic_sender();
                    tokio::spawn(async move {
                        match sender.send_message(msg.as_str(), opts).await {
                            Ok(_) => log::info!("Message sent successfully"),
                            Err(e) => log::error!("Error sending message: {:?}", e),
                        }
                    });
                } else {
                    log::error!("Client not initialized");
                }
                Ok(())
            }
        }
    }
}

pub struct PythonServiceBusClient {
    conn_str: String,
    topic: String,
    subscription: String,
    server_handle: Option<JoinHandle<()>>,
}

impl Default for PythonServiceBusClient {
    fn default() -> Self {
        PythonServiceBusClient::new(
            "Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;",
            "test-topic",
            "test-sub",
        )
    }
}

impl PythonServiceBusClient {
    pub fn new(
        conn_str: impl Into<String>,
        topic: impl Into<String>,
        subscription: impl Into<String>,
    ) -> Self {
        Self {
            conn_str: conn_str.into(),
            topic: topic.into(),
            subscription: subscription.into(),
            server_handle: None,
        }
    }

    pub fn start(&mut self) -> VoidRes {
        let conn_str = self.conn_str.clone();
        let topic_name = self.topic.clone();
        let subscription_name = self.subscription.clone();

        self.server_handle = Some(tokio::spawn(async move {
            loop {
                let conn_str = conn_str.clone();
                let topic_name = topic_name.clone();
                let subscription_name = subscription_name.clone();
                // Run in a separate thread since Python GIL can block
                let result = std::thread::spawn(move || {
                    Python::with_gil(|py| {
                        // Import required modules
                        // let azure_sb = py.import("azure.servicebus")?;
                        // let service_bus_client = azure_sb.getattr("ServiceBusClient")?;
                        //
                        // // Create client from connection string
                        // let client = service_bus_client
                        //     .call_method1("from_connection_string", (conn_str.clone(),))?;
                        // let dict = PyDict::new(py);
                        // dict.set_item("topic_name", topic_name.clone())?;
                        // dict.set_item("subscription_name", subscription_name.clone())?;
                        // dict.set_item("max_wait_time", 5)?;
                        //
                        // let receiver = client.call_method(
                        //     "get_subscription_receiver",
                        //     PyTuple::empty(py),
                        //     Some(&dict),
                        // )?;
                        //
                        // let dict = PyDict::new(py);
                        // dict.set_item("max_message_count", 10)?;
                        // dict.set_item("max_wait_time", 30)?;
                        // loop {
                        //     // Receive messages
                        //     let messages = receiver.call_method(
                        //         "receive_messages",
                        //         PyTuple::empty(py),
                        //         Some(&dict),
                        //     )?;
                        //
                        //     // Process messages
                        //     let message_count = messages.len()?;
                        //     if message_count == 0 {
                        //         // No messages received, briefly release GIL to allow other Python code to run
                        //         py.allow_threads(|| {
                        //             std::thread::sleep(std::time::Duration::from_millis(100));
                        //         });
                        //         continue;
                        //     }
                        //
                        //     for message in messages.iter()? {
                        //         let msg = message?;
                        //         let body = msg.getattr("body")?.extract::<Vec<u8>>()?;
                        //         let body_str = String::from_utf8_lossy(&body);
                        //
                        //         println!("Received message: {}", body_str);
                        //
                        //         // Complete the message to remove it from the queue
                        //         receiver.call_method1("complete_message", (msg,))?;
                        //     }
                        // }

                        // Close the client
                        // client.call_method0("close")?;
                        //
                        Ok::<(), PyErr>(())
                    })
                })
                .join()
                .unwrap();

                if let Err(e) = result {
                    log::error!("Python error: {}", e);
                }

                // Add a brief pause to avoid excessive CPU usage
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }));

        Ok(())
    }

    pub fn send_message(&self, message: &str) -> VoidRes {
        let conn_str = self.conn_str.clone();
        let topic_name = self.topic.clone();
        let message = message.to_string();

        // Send message in a separate thread to avoid blocking
        std::thread::spawn(move || {
            Python::with_gil(|py| -> PyResult<()> {
                // Import required modules
                let azure_sb = py.import("azure.servicebus")?;
                let service_bus_client = azure_sb.getattr("ServiceBusClient")?;
                let service_bus_message = azure_sb.getattr("ServiceBusMessage")?;

                // Create client from connection string
                let client =
                    service_bus_client.call_method1("from_connection_string", (conn_str,))?;

                // Get a topic sender
                let sender = client.call_method1("get_topic_sender", (topic_name,))?;

                // Create and send a message
                let msg = service_bus_message.call1((message,))?;
                sender.call_method1("send_messages", (msg,))?;

                // Close the client
                client.call_method0("close")?;

                Ok(())
            })
        });

        Ok(())
    }

    pub fn stop(&mut self) -> VoidRes {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            log::info!("Python Service Bus client stopped");
        }
        Ok(())
    }
}

impl Server<AzureMessage> for PythonServiceBusClient {
    fn start(&mut self) -> VoidRes {
        log::info!("Starting Python Service Bus Client");
        self.start()
    }

    fn stop(&mut self) -> VoidRes {
        log::info!("Stopping Python Service Bus Client");
        self.stop()
    }

    fn process(&mut self, message: AzureMessage) -> VoidRes {
        match message {
            AzureMessage::Start => {
                log::info!("Python Service Bus Client started");
                self.start()
            }
            AzureMessage::Stop => {
                log::info!("Python Service Bus Client stopped");
                self.stop()
            }
            AzureMessage::SendMessage(msg, _) => {
                log::info!("Sending message: {}", msg);
                self.send_message(&msg)
            }
        }
    }
}

pub enum AzureMessage {
    Start,
    Stop,
    SendMessage(String, Option<SendMessageOptions>),
}
