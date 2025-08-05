// #[cfg(test)]
// mod tests;
//
// use crate::VoidRes;
// use crate::actors::Actor;
// use crate::actors::workers::periodic::{PeriodicWorker, WorkerTaskHandler};
// use crate::error::KernelError;
// use sqlx::{Database, Postgres, Sqlite, SqlitePool};
// use std::time::Duration;
//
// #[derive(Clone)]
// pub struct DbWorker<DB: Database, H: DbTaskHandler<DB>> {
//     id: String,
//     periodic_worker: PeriodicWorker<PeriodicTaskHandler<H, DB>>,
//     pool: sqlx::Pool<DB>,
// }
//
// impl<DB: Database, H: DbTaskHandler<DB>> DbWorker<DB, H> {
//     pub fn new(id: String, task_handler: H, duration: Duration, pool: sqlx::Pool<DB>) -> Self {
//         Self {
//             id,
//             periodic_worker: PeriodicWorker::new(
//                 PeriodicTaskHandler::new(task_handler, pool.clone()),
//                 duration,
//             ),
//             pool,
//         }
//     }
// }
//
// struct PeriodicTaskHandler<H: DbTaskHandler<DB>, DB: Database> {
//     task_handler: H,
//     pool: sqlx::Pool<DB>,
// }
//
// impl<H: DbTaskHandler<DB>, DB: Database> PeriodicTaskHandler<H, DB> {
//     pub fn new(task_handler: H, pool: sqlx::Pool<DB>) -> Self {
//         Self { task_handler, pool }
//     }
// }
//
// impl<H: DbTaskHandler<DB>, DB: Database> Clone for PeriodicTaskHandler<H, DB> {
//     fn clone(&self) -> Self {
//         Self {
//             task_handler: self.task_handler.clone(),
//             pool: self.pool.clone(),
//         }
//     }
// }
//
// impl<H: DbTaskHandler<DB>, DB: Database> WorkerTaskHandler for PeriodicTaskHandler<H, DB> {
//     async fn handle(&mut self) -> VoidRes {
//         self.task_handler.handle(&self.pool).await?;
//         Ok(())
//     }
// }
//
// pub trait DbTaskHandler<DB: Database>: Clone + Send + 'static {
//     fn handle(
//         &mut self,
//         pool: &sqlx::Pool<DB>,
//     ) -> impl Future<Output = Result<(), sqlx::Error>> + Send;
// }
//
// enum DbMessage {
//     Stop,
//     Query(String),
// }
//
// impl<H> Actor<DbMessage> for DbWorker<Sqlite, H>
// where
//     H: DbTaskHandler<Sqlite>,
// {
//     fn id(&self) -> String {
//         self.id.clone()
//     }
//
//     async fn start(&mut self) -> VoidRes {
//         self.periodic_worker._start().await
//     }
//
//     async fn stop(&mut self) -> VoidRes {
//         self.pool.close().await;
//         self.periodic_worker._stop().await
//     }
//
//     async fn process(&mut self, message: DbMessage) -> VoidRes {
//         match message {
//             DbMessage::Stop => {
//                 log::info!("Stopping DB Worker");
//                 self.stop().await
//             }
//             DbMessage::Query(query) => {
//                 log::info!("Processing query: {}", query);
//                 sqlx::query(&query).execute(&self.pool).await?;
//                 Ok::<(), KernelError>(())
//             }
//         }
//     }
// }
