use crate::servers::ServerError;
use std::net::AddrParseError;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum KernelError {
    ServerError(ServerError),
    ChannelError(String),
}

unsafe impl Send for KernelError {}

impl From<ServerError> for KernelError {
    fn from(error: ServerError) -> Self {
        KernelError::ServerError(error)
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
