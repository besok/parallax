use actix::{Actor, AsyncContext, Context, Handler, Recipient, WrapFuture};
use actor::{ActorResultVoid, ActorServiceMessage};
use fe2o3_amqp::types::messaging::FromBody;
use fe2o3_amqp::{Connection, Receiver, Session};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender as TSender;
use tokio::time::sleep;

#[derive(Debug)]
pub struct AzureTopicListener<AzureMes, ActorMes>
where
    ActorMes: actix::Message + Send + Clone + Unpin + From<AzureMes> + 'static,
    <ActorMes as actix::Message>::Result: Send,
{
    key: String,
    url: String,
    topic: String,
    subscription: String,
    processors: Vec<Recipient<ActorMes>>,
    shutdown: Option<TSender<()>>,
    _phantom: std::marker::PhantomData<AzureMes>,
}

impl<AzureM, ActorM> Default for AzureTopicListener<AzureM, ActorM>
where
    ActorM: actix::Message + Send + Clone + Unpin + From<AzureM> + 'static,
    <ActorM as actix::Message>::Result: Send,
{
    fn default() -> Self {
        AzureTopicListener::new(
            "AzureTopicListener",
            "amqp://localhost:5672",
            "topic",
            "subscription",
            vec![],
        )
    }
}

impl<AzureM, ActorM> AzureTopicListener<AzureM, ActorM>
where
    ActorM: actix::Message + Send + Clone + Unpin + From<AzureM> + 'static,
    <ActorM as actix::Message>::Result: Send,
{
    pub fn new<T: Into<String>>(
        key: T,
        url: T,
        topic: T,
        subscription: T,
        processors: Vec<Recipient<ActorM>>,
    ) -> Self {
        AzureTopicListener {
            key: key.into(),
            url: url.into(),
            topic: topic.into(),
            subscription: subscription.into(),
            processors,
            shutdown: None,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<AzureM, ActorM> Handler<ActorServiceMessage> for AzureTopicListener<AzureM, ActorM>
where
    AzureM:
        for<'de> serde::Deserialize<'de> + Clone + Send + for<'de> FromBody<'de> + Unpin + 'static,
    ActorM: actix::Message + Send + Clone + Unpin + From<AzureM> + 'static,
    <ActorM as actix::Message>::Result: Send,
{
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ActorServiceMessage::Start => {
                log::info!("Already started, ignoring ...");
            }
            ActorServiceMessage::Stop => {
                log::info!("trying stopping actor");
                if let Some(shutdown) = self.shutdown.take() {
                    log::info!("Stopping actor");
                    shutdown.send(()).unwrap();
                }
            }
        }
        Ok(())
    }
}

impl<AzureM, ActorM> Actor for AzureTopicListener<AzureM, ActorM>
where
    AzureM:
        for<'de> serde::Deserialize<'de> + Clone + Send + for<'de> FromBody<'de> + Unpin + 'static,
    ActorM: actix::Message + Send + Clone + Unpin + From<AzureM> + 'static,
    <ActorM as actix::Message>::Result: Send,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let key = self.key.clone();
        let url = self.url.clone();
        let topic = self.topic.clone();
        let subscription = self.subscription.clone();
        let processors = self.processors.clone();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        self.shutdown = Some(shutdown_tx);
        ctx.spawn(
            async move {
                loop {
                    match start_listener::<AzureM, ActorM>(
                        key.clone(),
                        url.clone(),
                        topic.clone(),
                        subscription.clone(),
                        processors.clone(),
                    )
                    .await
                    {
                        Ok(_) => log::info!("Listener disconnected, retrying ..."),
                        Err(e) => {
                            log::error!("Error: {}, retrying in 5s...", e);
                            sleep(Duration::from_secs(5)).await;
                        }
                    }
                    if shutdown_rx.try_recv().is_ok() {
                        log::info!("Shutting down...");
                        break;
                    }
                }
            }
            .into_actor(self),
        );
    }
}

async fn start_listener<AzureM, ActorM>(
    key: String,
    url: String,
    topic: String,
    sub: String,
    processors: Vec<Recipient<ActorM>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    AzureM: for<'de> serde::Deserialize<'de> + Clone + Send + for<'de> FromBody<'de>,
    ActorM: actix::Message + Send + Clone + Unpin + From<AzureM> + 'static,
    <ActorM as actix::Message>::Result: Send,
{
    let mut connection = Connection::builder()
        .container_id(key)
        .open(url.as_str())
        .await?;

    let mut session = Session::begin(&mut connection).await?;

    let mut receiver = Receiver::attach(
        &mut session,
        "listener-link",
        &format!("{}/Subscriptions/{}", topic, sub),
    )
    .await?;

    log::info!("Listening for messages...");

    loop {
        match receiver.recv::<AzureM>().await {
            Ok(delivery) => {
                receiver.accept(&delivery).await?;
                let actor_mes: ActorM = delivery.into_body().into();
                for processor in &processors {
                    processor.do_send(actor_mes.clone());
                }
            }
            Err(e) => {
                eprintln!("Receive error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
