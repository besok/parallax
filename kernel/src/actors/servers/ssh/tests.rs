use crate::actors::servers::ssh::{SshMessage, SshServer};
use crate::actors::spawn_actor;
use crate::{Res, VoidRes, init_logger};
use russh::{ChannelMsg, client};
use russh_keys::key::PublicKey;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

struct TestSshClient;

impl TestSshClient {
    async fn call(&self, cmd: &str) -> Res<String> {
        let mut session = client::connect(
            Arc::new(client::Config::default()),
            ("127.0.0.1", 2222),
            TestSshClient,
        )
        .await?;

        let auth_success = session
            .authenticate_password("test_user", "test_pass")
            .await?;

        let mut channel = session.channel_open_session().await?;
        channel.exec(true, cmd).await?;

        let mut output = String::new();

        loop {
            let Some(msg) = channel.wait().await else {
                break;
            };

            match msg {
                ChannelMsg::Data { ref data } => {
                    if let Ok(str_data) = std::str::from_utf8(data) {
                        output.push_str(str_data);
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    assert_eq!(exit_status, 0, "Command exited with non-zero status");
                }
                _ => {}
            }
        }
        Ok(output)
    }
}

#[async_trait::async_trait]
impl client::Handler for TestSshClient {
    type Error = russh::Error;

    async fn check_server_key(
        self,
        server_public_key: &PublicKey,
    ) -> Result<(Self, bool), Self::Error> {
        Ok((self, true))
    }
}

#[tokio::test]
async fn smoke_ssh() -> VoidRes {
    init_logger();
    let server_handle = spawn_actor(SshServer::default(), None)?;
    sleep(Duration::from_millis(100)).await;

    let client = TestSshClient;

    assert_eq!("No files found\n", client.call("ls").await?);

    server_handle
        .send(SshMessage::AddFile {
            path: "C:\\Users\\besok\\Documents\\test1".to_string(),
            content: b"test".to_vec(),
        })
        .await?;
    server_handle
        .send(SshMessage::AddFile {
            path: "C:\\Users\\besok\\Documents\\test2".to_string(),
            content: b"test".to_vec(),
        })
        .await?;

    assert_eq!(
        "C:\\Users\\besok\\Documents\\test1\nC:\\Users\\besok\\Documents\\test2\n",
        client.call("ls").await?
    );

    server_handle
        .send(SshMessage::RemoveFile {
            path: "C:\\Users\\besok\\Documents\\test1".to_string(),
        })
        .await?;

    assert_eq!(
        "C:\\Users\\besok\\Documents\\test2\n",
        client.call("ls").await?
    );

    assert_eq!(
        "It is an Ssh test server!\n",
        client.call("ssh_test_server").await?
    );

    server_handle
        .send(SshMessage::AddProcessor(Box::new(|cmd, files| {
            if cmd.trim() == "ssh_test_server" {
                Some(Ok(("It is a new Ssh test server!\n".to_string(), files)))
            } else {
                None
            }
        })))
        .await?;

    assert_eq!(
        "It is a new Ssh test server!\n",
        client.call("ssh_test_server").await?
    );

    server_handle.send(SshMessage::Stop).await?;
    sleep(Duration::from_millis(100)).await;
    Ok(())
}
