mod http;

use crate::error::KernelError;
use crate::{Res, VoidRes};
use tokio::sync::mpsc::{self, Sender};
use tokio::task;

type ServerId = String;

#[derive(Debug)]
pub enum ServerError {
    StartError(String, ServerId),
    RuntimeError(String, ServerId),
    StopError(String, ServerId),
    ClientError(String),
}
pub struct ServerHandle<Mes> {
    sender: Sender<Mes>,
}

impl<Mes> ServerHandle<Mes> {
    pub fn new(sender: Sender<Mes>) -> Self {
        ServerHandle { sender }
    }

    pub async fn send(&self, message: Mes) -> VoidRes {
        Ok(self.sender.send(message).await?)
    }
}

pub trait Server<Mes> {
    fn start(&mut self) -> VoidRes;
    fn stop(&mut self) -> VoidRes;
    fn process(&mut self, message: Mes) -> VoidRes;
}

pub fn spawn_server<M, Serv>(
    mut server: Serv,
    err_sender: Option<Sender<KernelError>>,
) -> Res<ServerHandle<M>>
where
    Serv: Server<M> + Send + 'static,
    M: Send + 'static,
{
    let (sender, mut receiver) = mpsc::channel::<M>(32);
    task::spawn(async move {
        if let Err(e) = server.start() {
            if let Some(err_sender) = err_sender {
                let _ = err_sender.send(e).await;
            }
            return;
        }
        loop {
            tokio::select! {
                Some(message) = receiver.recv() => {
                    if let Err(e) = server.process(message) {
                        if let Some(ref err_sender) = err_sender {
                            let _ = err_sender.send(e).await;
                        }
                    }
                }
                else => break,
            }
        }
        if let Err(e) = server.stop() {
            if let Some(err_sender) = err_sender {
                let _ = err_sender.send(e).await;
            }
        }
    });

    Ok(ServerHandle::new(sender))
}

mod tests {
    use crate::servers::http::{BaseHttpServer, HttpMessage};
    use crate::servers::{ServerError, spawn_server};
    use crate::{VoidRes, init_logger};
    use serde_json::Value;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_http_server() -> VoidRes {
        init_logger();

        let server_handle = spawn_server(BaseHttpServer::default(), None)?;

        let client = reqwest::Client::new();
        let response = client
            .get("http://127.0.0.1:8080/health")
            .send()
            .await
            .map_err(|e| ServerError::ClientError(e.to_string()))?;

        assert_eq!(response.status(), 200);

        let body: Value = response
            .json()
            .await
            .map_err(|e| ServerError::ClientError(e.to_string()))?;
        assert_eq!(body["status"], "up");

        server_handle.sender.send(HttpMessage::Stop).await?;

        sleep(Duration::from_millis(100)).await;

        Ok(())
    }
}
