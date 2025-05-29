use crate::actors::servers::azure;
use crate::actors::servers::azure::AzureMessage;
use crate::actors::spawn_actor;
use crate::{VoidRes, init_logger};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn smoke_client() -> VoidRes {
    init_logger();
    let serv = azure::AzureTopicClient::default();

    let serv_handle = spawn_actor(serv, None)?;
    sleep(Duration::from_millis(100)).await;
    serv_handle
        .send(AzureMessage::SendMessage("test".as_bytes().to_vec()))
        .await?;

    serv_handle.send(AzureMessage::Stop).await?;
    sleep(Duration::from_millis(100)).await;

    Ok(())
}
