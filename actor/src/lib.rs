use actix::{Actor, Context, Handler, Message};
use log::log;
use std::fmt::Display;

#[derive(Debug)]
pub enum ActorError {
    StartupError(String),
    RuntimeError(String),
    ShutdownError(String),
}
pub type ActorResult<T> = Result<T, ActorError>;
pub type ActorResultVoid = Result<(), ActorError>;

#[derive(Debug, Message)]
#[rtype(result = "ActorResultVoid")]
pub enum ActorServiceMessage {
    Start,
    Stop,
}

impl From<String> for ActorError {
    fn from(s: String) -> Self {
        ActorError::RuntimeError(s)
    }
}

pub struct EchoActor;

impl Actor for EchoActor {
    type Context = Context<Self>;
}

impl<T: Display + actix::Message<Result = ()>> Handler<T> for EchoActor {
    type Result = ();

    fn handle(&mut self, msg: T, ctx: &mut Self::Context) -> Self::Result {
        log::info!("[Echo] {}", msg);
    }
}
