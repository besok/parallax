use actix::{
    Actor, ActorContext, ActorFutureExt, AsyncContext, Context, Handler, Message as ActixMessage,
    Message, ResponseActFuture, WrapFuture,
};
use actor::{ActorError, ActorResultVoid, ActorServiceMessage};
use fe2o3_amqp::connection::ConnectionHandle;
use fe2o3_amqp::session::SessionHandle;
use fe2o3_amqp::types::messaging::{Body, IntoBody, Outcome};
use fe2o3_amqp::{Connection, Sender, Session};
use log::log;
use std::collections::VecDeque;
use std::fmt::Debug;

#[derive(Debug)]
pub struct AzureTopicSender<AzureM>
where
    AzureM: IntoBody + Send + 'static,
{
    key: String,
    url: String,
    topic: String,
    connection: Option<ConnectionHandle<()>>,
    session: Option<SessionHandle<()>>,
    sender: Option<Sender>,
    message_queue: VecDeque<AzureM>,
    is_processing: bool,
    _phantom: std::marker::PhantomData<AzureM>,
}

impl<AzureM> AzureTopicSender<AzureM>
where
    AzureM: IntoBody + Send + 'static + Clone,
{
    pub fn new<T: Into<String>>(key: T, url: T, topic: T) -> Self {
        AzureTopicSender {
            key: key.into(),
            url: url.into(),
            topic: topic.into(),
            connection: None,
            session: None,
            sender: None,
            message_queue: VecDeque::new(),
            is_processing: false,
            _phantom: std::marker::PhantomData,
        }
    }

    async fn establish_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!(
            "[{}] Establishing AMQP connection to {}",
            self.key,
            self.url
        );

        let mut connection = Connection::builder()
            .container_id(&format!("{}", self.key))
            .open(self.url.as_str())
            .await?;

        let mut session = Session::begin(&mut connection).await?;

        let sender = Sender::attach(
            &mut session,
            &format!("{}-sender-link", self.key),
            &self.topic,
        )
        .await?;

        self.connection = Some(connection);
        self.session = Some(session);
        self.sender = Some(sender);

        log::info!("[{}] Connected to topic: {}", self.key, self.topic);
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<(), String> {
        log::info!("[{}] Attempting to reconnect...", self.key);

        self.connection = None;
        self.session = None;
        self.sender = None;

        self.establish_connection()
            .await
            .map_err(|e| format!("Reconnection failed: {}", e))
    }

    fn is_connected(&self) -> bool {
        self.connection.is_some() && self.session.is_some() && self.sender.is_some()
    }

    async fn try_send(key: String, message: AzureM, mut sender: Sender) -> Result<(), String> {
        let outcome: Outcome = sender
            .send(message)
            .await
            .map_err(|e| format!("Send failed: {}", e))?;

        outcome
            .accepted_or_else(|state| state)
            .map_err(|e| format!("Message not accepted: {:?}", e))?;

        log::info!("[{}] Message sent successfully", key);
        Ok(())
    }
}

impl<AzureM> Actor for AzureTopicSender<AzureM>
where
    AzureM: IntoBody + Send + Unpin + Clone + 'static,
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("[{}] Azure sender actor started", self.key);

        // Establish connection on startup
        let key = self.key.clone();
        let url = self.url.clone();
        let topic = self.topic.clone();

        ctx.spawn(
            async move {
                log::info!("[{}] Establishing AMQP connection to {}", key, url);

                let mut connection = Connection::builder()
                    .container_id(&format!("{}", key))
                    .open(url.as_str())
                    .await
                    .map_err(|e| format!("Connection failed: {}", e))?;

                let mut session = Session::begin(&mut connection)
                    .await
                    .map_err(|e| format!("Session failed: {}", e))?;

                let sender = Sender::attach(&mut session, &format!("{}-sender-link", key), &topic)
                    .await
                    .map_err(|e| format!("Sender attach failed: {}", e))?;

                Ok((connection, session, sender))
            }
            .into_actor(self)
            .map(
                |result: Result<(ConnectionHandle<()>, SessionHandle<()>, Sender), String>,
                 actor: &mut AzureTopicSender<_>,
                 _ctx| {
                    match result {
                        Ok((connection, session, sender)) => {
                            actor.connection = Some(connection);
                            actor.session = Some(session);
                            actor.sender = Some(sender);
                            log::info!(
                                "[{}] Initial connection established successfully",
                                actor.key
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "[{}] Failed to establish initial connection: {}",
                                actor.key,
                                e
                            );
                            // Could implement retry logic here
                        }
                    }
                },
            ),
        );
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        log::info!(
            "[{}] Azure sender actor stopping, cleaning up connections",
            self.key
        );

        self.connection = None;
        self.session = None;
        self.sender = None;

        log::info!("[{}] Azure sender actor stopped", self.key);
        actix::Running::Stop
    }
}

impl<AzureM> Handler<ActorServiceMessage> for AzureTopicSender<AzureM>
where
    AzureM: IntoBody + Send + Unpin + Clone + 'static,
{
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ActorServiceMessage::Start => {
                log::info!("[{}] Start message received (already started)", self.key);
            }
            ActorServiceMessage::Stop => {
                log::info!("[{}] Stop message received", self.key);
                ctx.stop();
            }
        }
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "ActorResultVoid")]
pub struct SendMessage<AzureM>(pub AzureM);

impl<AzureM> Handler<SendMessage<AzureM>> for AzureTopicSender<AzureM>
where
    AzureM: IntoBody + Send + Unpin + Clone + 'static,
{
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: SendMessage<AzureM>, ctx: &mut Self::Context) -> Self::Result {
        if !self.is_connected() {
            return Err(ActorError::RuntimeError(
                "Connection not established".to_string(),
            ));
        }
        let sender = self.sender.take().unwrap();
        let key = self.key.clone();
        let _ = Box::pin(
            ctx.spawn(
                async move {
                    let _ = Self::try_send(key.clone(), msg.0.clone(), sender).await;
                }
                .into_actor(self),
            ),
        );

        Ok(())
    }
}
