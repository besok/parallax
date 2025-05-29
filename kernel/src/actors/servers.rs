pub mod azure;
pub mod http;
pub mod opcua;
pub mod ssh;

use std::sync::PoisonError;

#[derive(Debug)]
pub enum ServerError {
    StartError(String, ServerId),
    RuntimeError(String),
    ClientError(String),
}
impl<T> From<PoisonError<T>> for ServerError {
    fn from(error: PoisonError<T>) -> Self {
        ServerError::RuntimeError(error.to_string())
    }
}
impl From<russh::Error> for ServerError {
    fn from(e: russh::Error) -> Self {
        ServerError::ClientError(e.to_string())
    }
}

type ServerId = String;
