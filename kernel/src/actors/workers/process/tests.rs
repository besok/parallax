// use crate::actors::workers::process::{ProcessWorker, ProcessWorkerMessage};
// use crate::actors::{spawn_actor_with, spawn_actor};
// use crate::{VoidRes, init_logger};
//
// #[tokio::test]
// async fn test_process() -> VoidRes {
//     init_logger();
//     let w = ProcessWorker::new(
//         "Test",
//         "python3",
//         "--version",
//         vec![
//         ],
//     );
//
//     let w01 = spawn_actor(w).await?;
//
//
//
//     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//
//     w01.send(ProcessWorkerMessage::Stop).await?;
//     Ok(())
// }
