use crate::actors::servers::ServerError;
use bevy::ecs::query::QuerySingleError;
use std::net::AddrParseError;
use std::sync::PoisonError;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum KernelError {
    ServerError(ServerError),
    ChannelError(String),
    SystemError(String),
}

impl From<QuerySingleError> for KernelError {
    fn from(value: QuerySingleError) -> Self {
        KernelError::SystemError(value.to_string())
    }
}

impl From<azure_core::Error> for KernelError {
    fn from(error: azure_core::Error) -> Self {
        KernelError::ServerError(ServerError::RuntimeError(error.to_string()))
    }
}

impl From<sqlx::Error> for KernelError {
    fn from(error: sqlx::Error) -> Self {
        KernelError::SystemError(error.to_string())
    }
}

impl From<()> for KernelError {
    fn from(_: ()) -> Self {
        KernelError::ServerError(ServerError::ClientError("Empty error".to_string()))
    }
}

impl KernelError {
    pub fn client(v: &str) -> Self {
        KernelError::ServerError(ServerError::ClientError(v.to_string()))
    }
}

unsafe impl Send for KernelError {}

impl<T> From<PoisonError<T>> for KernelError {
    fn from(error: PoisonError<T>) -> Self {
        KernelError::SystemError(error.to_string())
    }
}

impl From<ServerError> for KernelError {
    fn from(error: ServerError) -> Self {
        KernelError::ServerError(error)
    }
}

impl From<std::io::Error> for KernelError {
    fn from(error: std::io::Error) -> Self {
        KernelError::SystemError(error.to_string())
    }
}

impl From<russh::Error> for KernelError {
    fn from(error: russh::Error) -> Self {
        let s: ServerError = error.into();
        s.into()
    }
}

pub struct ErrorHandler {
    receiver: Receiver<KernelError>,
    sender: Sender<KernelError>,
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self { receiver, sender }
    }

    pub fn get_sender(&self) -> Sender<KernelError> {
        self.sender.clone()
    }

    pub async fn handle_errors(&mut self) {
        while let Some(error) = self.receiver.recv().await {
            log::error!("Kernel error: {:?}", error);
        }
    }
}

impl<T> From<SendError<T>> for KernelError {
    fn from(value: SendError<T>) -> Self {
        KernelError::ChannelError(value.to_string())
    }
}
