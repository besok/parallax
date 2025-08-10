use actix::Message;

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
