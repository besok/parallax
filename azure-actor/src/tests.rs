use crate::listener::AzureTopicListener;
use crate::sender::{AzureTopicSender, SendMessage};
use actix::{Actor, Message};
use actor::{ActorError, ActorResultVoid, ActorServiceMessage, EchoActor};
use fe2o3_amqp::{Connection, Sender, Session};
use std::fmt::Display;
use std::time::Duration;
use tokio::time::sleep;
use utils::logger_on;

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
struct M(String);
impl From<String> for M {
    fn from(s: String) -> Self {
        M(s)
    }
}

impl Display for M {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[actix::test]
async fn azure_smoke() -> ActorResultVoid {
    logger_on();

    let sender: AzureTopicSender<String> =
        AzureTopicSender::new("Sender", "amqp://localhost:5672", "test-topic");

    let sender_actor = sender.start();
    sleep(Duration::from_secs(1)).await;
    let _ = sender_actor
        .send(SendMessage("Hello World".to_string()))
        .await;

    let l: AzureTopicListener<String, M> = AzureTopicListener::new(
        "Test",
        "amqp://localhost:5672",
        "test-topic",
        "test-sub",
        vec![EchoActor.start().recipient()],
    );

    let actor = l.start();

    sleep(Duration::from_secs(5)).await;

    sleep(Duration::from_secs(3)).await;

    let _ = actor.send(ActorServiceMessage::Stop).await;

    sleep(Duration::from_secs(3)).await;

    Ok(())
}
