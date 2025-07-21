use bevy::prelude::{Bundle, Component};
use crate::error::KernelError;
use crate::{Res, VoidRes};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task;

pub mod servers;
pub mod workers;
pub mod tags;

#[derive(Component)]
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
    fn start(&mut self) -> impl Future<Output = VoidRes> + Send;
    fn stop(&mut self) -> impl Future<Output = VoidRes> + Send;
    fn process(&mut self, message: Mes) -> impl Future<Output = VoidRes> + Send;
}

pub async fn spawn_actor_with<M, A>(
    mut actor: A,
    err_sender: Option<Sender<KernelError>>,
) -> Res<ActorHandle<M>>
where
    A: Actor<M> + Send + 'static,
    M: Send + 'static,
{
    let (sender, mut receiver) = mpsc::channel::<M>(32);
    if let Err(e) = actor.start().await {
        let msg = format!("Failed to start server: {:?}", e);
        if let Some(err_sender) = err_sender {
            let _ = err_sender.send(e).await;
        }
        return Err(KernelError::SystemError(msg));
    }
    tokio::spawn(async move {
        loop {
            tokio::select! {
                    Some(message) = receiver.recv() => {
                        if let Err(e) = actor.process(message).await {
                            if let Some(ref err_sender) = err_sender {
                                let _ = err_sender.send(e).await;
                            }
                        }
                    }
                      else => {
                       let _ = actor.stop().await;
                        break;
                }
            }
        }
    });

    Ok(ActorHandle::new(sender))
}

pub async fn spawn_actor<M, A>(actor: A) -> Res<ActorHandle<M>>
where
    A: Actor<M> + Send + 'static,
    M: Send + 'static,
{
    spawn_actor_with(actor, None).await
}


mod tests {
    use crate::actors::servers::ServerError;
    use crate::actors::servers::http::{BaseHttpServer, HttpMessage};
    use crate::actors::spawn_actor_with;
    use crate::{VoidRes, init_logger};
    use serde_json::Value;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_http_server() -> VoidRes {
        init_logger();

        let server_handle = spawn_actor_with(BaseHttpServer::default(), None).await?;

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
