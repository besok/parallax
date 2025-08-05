use crate::actors::servers::azure;
use crate::actors::servers::azure::{AzureMessage, AzureTopicListener, EchoServiceBusMsgHandler};
use crate::{VoidRes, init_logger};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn smoke_client() -> VoidRes {
    init_logger();
    let serv: AzureTopicListener<EchoServiceBusMsgHandler> = AzureTopicListener::default();

    // let serv_handle = spawn_actor(serv, None).await?;
    //
    // sleep(Duration::from_millis(1000)).await;
    //
    // serv_handle
    //     .send(AzureMessage::SendMessage(b"Hello, Azure!".to_vec()))
    //     .await?;
    // sleep(Duration::from_millis(10000)).await;
    //
    // serv_handle.send(AzureMessage::Stop).await?;

    Ok(())
}
