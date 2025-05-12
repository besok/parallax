use super::*;
use crate::{init_logger, servers::spawn_server};
use reqwest::Client;

#[tokio::test]
async fn test_azure_servicebus_server() -> VoidRes {
    init_logger();

    let server = AzureServiceBusServer::default();
    let server_handle = spawn_server(server, None)?;

    // Give the server time to start
    sleep(Duration::from_secs(1000)).await;

    // Test HTTP management API
    let client = Client::new();

    // Test health endpoint
    let resp = client
        .get("http://127.0.0.1:5672/health")
        .send()
        .await
        .map_err(|e| ServerError::ClientError(e.to_string()))?;

    assert_eq!(resp.status().as_u16(), 200);

    // Test connection string endpoint
    let resp = client
        .get("http://127.0.0.1:5672/connection-string")
        .send()
        .await
        .map_err(|e| ServerError::ClientError(e.to_string()))?;

    assert_eq!(resp.status().as_u16(), 200);

    // Setup test queues and topics
    server_handle
        .send(AzureServiceBusCommand::AddQueue("test-queue".to_string()))
        .await?;
    server_handle
        .send(AzureServiceBusCommand::AddTopic(
            "test-topic".to_string(),
            vec!["sub1".to_string()],
        ))
        .await?;
    let mes = ServiceBusMessage {
        message_id: Some(format!("msg-{}", uuid::Uuid::new_v4())),
        body: "data".as_bytes().to_vec(),
        properties: HashMap::new(),
        content_type: Some("application/octet-stream".to_string()),
        subject: None,
    };
    server_handle
        .send(AzureServiceBusCommand::PublishMessage(
            "test-topic".to_string(),
            mes,
        ))
        .await?;
    sleep(Duration::from_millis(100)).await;

    // Shutdown server
    server_handle.send(AzureServiceBusCommand::Stop).await?;
    sleep(Duration::from_millis(100)).await;
    Ok(())
}

#[tokio::test]
async fn test_azure_servicebus_server_py() -> VoidRes {
    init_logger();

    let server = AzureServiceBusServer::default();
    let server_handle = spawn_server(server, None)?;

    // Give the server time to start

    server_handle
        .send(AzureServiceBusCommand::AddTopic(
            "test-topic".to_string(),
            vec!["sub1".to_string()],
        ))
        .await?;
    let mes = ServiceBusMessage {
        message_id: Some(format!("msg-{}", uuid::Uuid::new_v4())),
        body: "data".as_bytes().to_vec(),
        properties: HashMap::new(),
        content_type: Some("application/octet-stream".to_string()),
        subject: None,
    };
    server_handle
        .send(AzureServiceBusCommand::PublishMessage(
            "test-topic".to_string(),
            mes,
        ))
        .await?;
    sleep(Duration::from_secs(1000)).await;
    sleep(Duration::from_millis(100)).await;

    // Shutdown server
    server_handle.send(AzureServiceBusCommand::Stop).await?;
    sleep(Duration::from_millis(100)).await;
    Ok(())
}
