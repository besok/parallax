use crate::actors::spawn_actor_with;
use crate::actors::workers::db::{DbMessage, DbTaskHandler, DbWorker};
use crate::{VoidRes, init_logger};
use sqlx::{Error, Pool, Row, Sqlite, SqlitePool};
use std::time::Duration;

#[derive(Clone)]
struct DataQuery;
impl DbTaskHandler<Sqlite> for DataQuery {
    async fn handle(&mut self, pool: &Pool<Sqlite>) -> Result<(), Error> {
        let tasks = sqlx::query("SELECT * FROM tasks").fetch_all(pool).await?;

        for task in tasks {
            let id: i64 = task.get("id");
            let description: String = task.get("description");
            let completed: bool = task.get("completed");
            let created_at: chrono::NaiveDateTime = task.get("created_at");

            log::info!(
                " -- Task ID: {}, Description: {}, Completed: {}, Created At: {}",
                id,
                description,
                completed,
                created_at
            );
        }

        Ok(())
    }
}

#[tokio::test]
async fn sqlite_smoke() -> VoidRes {
    init_logger();
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

    let handle = spawn_actor_with(
        DbWorker::new(DataQuery, Duration::from_secs(1), pool.clone()),
        None,
    )
    .await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    handle
        .send(DbMessage::Query(
            "insert into tasks (description, completed) values ('Test Task', false)".to_string(),
        ))
        .await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    handle.send(DbMessage::Stop).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
