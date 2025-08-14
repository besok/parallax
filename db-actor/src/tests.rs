use crate::error::SqlResult;
use crate::sqlite::SqLiteQueryActor;
use actix::{Actor, AsyncContext, Context, Handler, Message, WrapFuture};
use sqlx::sqlite::SqliteRow;
use sqlx::types::chrono;
use sqlx::{Row, Sqlite, SqlitePool};
use std::time::Duration;
use utils::logger_on;

#[derive(Clone)]
struct DataQuery;

#[derive(Debug, Message, Clone)]
#[rtype(result = "()")]
struct DataQueryResult(Vec<String>);

impl From<Vec<SqliteRow>> for DataQueryResult {
    fn from(value: Vec<SqliteRow>) -> Self {
        let mut data = vec![];
        for task in value {
            let id: i64 = task.get("id");
            let description: String = task.get("description");
            let completed: bool = task.get("completed");
            let created_at: chrono::NaiveDateTime = task.get("created_at");

            data.push(format!(
                "Task ID: {}, Description: {}, Completed: {}, Created At: {}",
                id, description, completed, created_at
            ));
        }
        Self(data)
    }
}

struct DataQueryReceiver(usize);

impl Actor for DataQueryReceiver {
    type Context = Context<Self>;
}

impl Handler<DataQueryResult> for DataQueryReceiver {
    type Result = ();

    fn handle(&mut self, msg: DataQueryResult, ctx: &mut Self::Context) -> Self::Result {
        log::info!("{:?}", msg.0);
        self.0 += 1;
    }
}

#[actix::test]
async fn sqlite_smoke() -> SqlResult<()> {
    logger_on();
    let database_url = "sqlite::memory:";
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            description TEXT NOT NULL,
            completed BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query("INSERT INTO tasks (description, completed) VALUES (?, ?)")
        .bind("Sample Task")
        .bind(false)
        .execute(&pool)
        .await?;

    let mut actor: SqLiteQueryActor<DataQueryResult, _> = SqLiteQueryActor::new(
        "TaskQuerySqliteWorker".to_string(),
        || sqlx::query("SELECT * FROM tasks"),
        Duration::from_secs(1),
        pool.clone(),
    );

    actor.subscribe(DataQueryReceiver(0).start().recipient());
    actor.start();

    tokio::time::sleep(Duration::from_secs(2)).await;
    sqlx::query("INSERT INTO tasks (description, completed) VALUES (?, ?)")
        .bind("Sample Task2")
        .bind(false)
        .execute(&pool)
        .await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}
