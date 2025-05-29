use crate::error::KernelError;
use crate::{Res, VoidRes};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task;

pub mod servers;
mod workers;

pub struct ActorHandle<Mes> {
    sender: Sender<Mes>,
}

impl<Mes> ActorHandle<Mes> {
    pub fn new(sender: Sender<Mes>) -> Self {
        ActorHandle { sender }
    }

    pub async fn send(&self, message: Mes) -> VoidRes {
        Ok(self.sender.send(message).await?)
    }

    pub fn send_sync(&self, message: Mes) -> VoidRes {
        let sender = self.sender.clone();
        task::block_in_place(move || {
            sender
                .blocking_send(message)
                .map_err(|e| KernelError::ChannelError(e.to_string()))
        })
    }
}

pub trait Actor<Mes> {
    fn start(&mut self) -> VoidRes;
    fn stop(&mut self) -> VoidRes;
    fn process(&mut self, message: Mes) -> VoidRes;
}

pub fn spawn_actor<M, Serv>(
    mut server: Serv,
    err_sender: Option<Sender<KernelError>>,
) -> Res<ActorHandle<M>>
where
    Serv: Actor<M> + Send + 'static,
    M: Send + 'static,
{
    let (sender, mut receiver) = mpsc::channel::<M>(32);
    tokio::spawn(async move {
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
    });

    Ok(ActorHandle::new(sender))
}

mod tests {
    use crate::actors::servers::ServerError;
    use crate::actors::servers::http::{BaseHttpServer, HttpMessage};
    use crate::actors::spawn_actor;
    use crate::{VoidRes, init_logger};
    use serde_json::Value;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_http_server() -> VoidRes {
        init_logger();

        let server_handle = spawn_actor(BaseHttpServer::default(), None)?;

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
