use crate::error::KernelError;
use crate::{Res, VoidRes};
use bevy::prelude::{Bundle, Commands, Component, Entity, Name};
use bevy::tasks::{IoTaskPool, Task};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task;

pub mod servers;
pub mod tags;
pub mod workers;

#[derive(Component)]
pub struct ActorHandle<Mes> {
    to_actor: Sender<Mes>,
    from_actor: Receiver<Mes>,
    entity: Entity,
}

impl<Mes> ActorHandle<Mes> {
    pub fn new(entity: Entity, sender: Sender<Mes>, receiver: Receiver<Mes>) -> Self {
        ActorHandle {
            to_actor: sender,
            from_actor: receiver,
            entity,
        }
    }

    pub async fn send(&self, message: Mes) -> VoidRes {
        Ok(self.to_actor.send(message).await?)
    }

    pub fn send_sync(&self, message: Mes) -> VoidRes {
        let sender = self.to_actor.clone();
        task::block_in_place(move || {
            sender
                .blocking_send(message)
                .map_err(|e| KernelError::ChannelError(e.to_string()))
        })
    }
}

pub trait ActorMessage {
    fn error(error: KernelError, actor_id: String) -> Self;
}

pub trait Actor<Mes: ActorMessage> {
    fn id(&self) -> String;
    fn start(&mut self) -> impl Future<Output = VoidRes> + Send;
    fn stop(&mut self) -> impl Future<Output = VoidRes> + Send;
    fn process(
        &mut self,
        message: Mes,
        sender: Sender<Mes>,
    ) -> impl Future<Output = VoidRes> + Send;
}
#[derive(Component)]
struct ActorTask(Task<()>);
pub async fn spawn_actor<M, A>(
    mut actor: A,
    commands: &mut Commands<'_, '_>,
    task_pool: &IoTaskPool,
) -> Entity
where
    A: Actor<M> + Send + 'static,
    M: ActorMessage + Send + 'static,
{
    let (to_actor_tx, mut to_actor_rx) = mpsc::channel::<M>(32);
    let (from_actor_tx, from_actor_rx) = mpsc::channel::<M>(32);

    let actor_id = actor.id();

    let task = task_pool.spawn(async move {
        if let Err(e) = actor.start().await {
            let _ = from_actor_tx.send(M::error(e, actor.id())).await;
            return;
        }

        while let Some(message) = to_actor_rx.recv().await {
            if let Err(e) = actor.process(message, from_actor_tx.clone()).await {
                let _ = from_actor_tx.send(M::error(e, actor.id())).await;
            }
        }

        if let Err(e) = actor.stop().await {
            let _ = from_actor_tx.send(M::error(e, actor.id())).await;
        }
    });

    let entity = commands.spawn_empty().id();

    let ah = ActorHandle::new(entity, to_actor_tx, from_actor_rx);
    commands
        .get_entity(entity)
        .unwrap()
        .insert((
            ah,
            ActorTask(task),
            Name::new(format!("Actor: {}", actor_id)),
        ))
        .id()
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

        let server_handle = spawn_actor(BaseHttpServer::default(), None).await?;

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

        server_handle.to_actor.send(HttpMessage::Stop).await?;

        sleep(Duration::from_millis(100)).await;

        Ok(())
    }
}
