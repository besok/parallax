use actor::ActorError;
use sqlx::Error;

#[derive(Debug)]
pub struct SqlError(Error);

pub type SqlResult<T> = Result<T, SqlError>;

impl From<Error> for SqlError {
    fn from(value: Error) -> Self {
        SqlError(value)
    }
}

impl From<SqlError> for ActorError {
    fn from(value: SqlError) -> Self {
        ActorError::RuntimeError(value.0.to_string())
    }
}
