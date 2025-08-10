use actor::ActorError;
use std::fmt::{Display, Formatter};
use std::sync::PoisonError;

#[derive(Debug)]
pub struct SshError(String);
pub type SshResult<T> = Result<T, SshError>;
pub type SshResultVoid = SshResult<()>;
impl From<russh::Error> for SshError {
    fn from(e: russh::Error) -> Self {
        SshError(e.to_string())
    }
}

impl Display for SshError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<SshError> for ActorError {
    fn from(value: SshError) -> Self {
        ActorError::RuntimeError(value.to_string())
    }
}

impl<T> From<PoisonError<T>> for SshError {
    fn from(error: PoisonError<T>) -> Self {
        SshError(error.to_string())
    }
}
