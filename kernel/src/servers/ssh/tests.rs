use crate::servers::spawn_server;
use crate::servers::ssh::SshServer;
use crate::{VoidRes, init_logger};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn smoke_ssh() -> VoidRes {
    init_logger();
    let serv = SshServer::default();
    spawn_server(serv, None)?;

    sleep(Duration::from_secs(1000)).await;
    Ok(())
}
