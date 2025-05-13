use crate::servers::azure::AzureMessage;
use crate::servers::http::HttpMessage;
use crate::servers::{azure, spawn_server};
use crate::{VoidRes, init_logger};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn smoke_client() -> VoidRes {
    init_logger();
    let serv = azure::AzureNativeTopicClient::default();

    let serv_handle = spawn_server(serv, None)?;
    sleep(Duration::from_millis(100)).await;
    serv_handle
        .send(AzureMessage::SendMessage("test".to_string(), None))
        .await?;

    sleep(Duration::from_millis(10000)).await;
    Ok(())
}

#[tokio::test]
async fn smoke_py_client() -> VoidRes {
    init_logger();
    let serv = azure::PythonServiceBusClient::default();

    let serv_handle = spawn_server(serv, None)?;
    sleep(Duration::from_millis(100)).await;
    serv_handle
        .send(AzureMessage::SendMessage("test".to_string(), None))
        .await?;

    sleep(Duration::from_millis(10000)).await;
    Ok(())
}
