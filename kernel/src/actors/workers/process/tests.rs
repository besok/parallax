use crate::actors::workers::process::{ProcessWorker, ProcessWorkerMessage};
use crate::actors::{spawn_actor, spawn_just_actor};
use crate::{VoidRes, init_logger};
#[tokio::test]
async fn test_process() -> VoidRes {
    init_logger();
    let w = ProcessWorker::new(
        "Test",
        "python3",
        "/Users/boris.zhgu",
        vec![("HTTP_PORT", "5001")],
    );

    let h = spawn_just_actor(w).await?;

    tokio::time::sleep(std::time::Duration::from_secs(600)).await;

    h.send(ProcessWorkerMessage::Stop).await?;
    Ok(())
}
