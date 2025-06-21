use crate::VoidRes;
use crate::actors::Actor;
use crate::error::KernelError;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct PeriodicWorker<Task: WorkerTaskHandler = DefaultWorkerTask> {
    stop_signal: Option<Sender<()>>,
    interval: Duration,
    task: Task,
}

pub trait WorkerTaskHandler: Clone + Send + 'static {
    fn handle(&mut self) -> impl Future<Output = VoidRes> + Send;
}
#[derive(Clone, Debug)]
struct DefaultWorkerTask;
impl WorkerTaskHandler for DefaultWorkerTask {
    async fn handle(&mut self) -> VoidRes {
        log::info!("Default worker task executed");
        Ok(())
    }
}

impl<T: WorkerTaskHandler> PeriodicWorker<T> {
    pub fn new(task: T, interval: Duration) -> Self {
        Self {
            stop_signal: None,
            interval,
            task,
        }
    }

    pub async fn _start(&mut self) -> VoidRes {
        let interval = self.interval;
        let mut interval_timer = tokio::time::interval(interval);
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);
        self.stop_signal = Some(stop_tx);

        let mut cloned_self = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        if let Err(e) = cloned_self.task.handle().await {
                            log::error!("The report error returns a failed state: {:?}", e);
                            break;
                        }
                    }
                    Some(_) = stop_rx.recv() => {
                        log::info!("Periodic worker received stop signal");
                        break;
                    }
                    else => break,
                }
            }
        });

        Ok(())
    }

    pub async fn _stop(&mut self) -> VoidRes {
        if let Some(tx) = &self.stop_signal {
            let _ = tx.try_send(());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::actors::workers::periodic::{DefaultWorkerTask, PeriodicWorker};
    use crate::actors::{Actor, spawn_actor_with};
    use crate::{VoidRes, init_logger};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::mpsc::Sender;

    #[derive(Clone)]
    struct EchoPeriodicWorker {
        delegate: PeriodicWorker,
        echo: Arc<Mutex<String>>,
    }

    impl EchoPeriodicWorker {
        pub fn new(interval: Duration, echo: String) -> Self {
            Self {
                delegate: PeriodicWorker::new(DefaultWorkerTask, interval),
                echo: Arc::new(Mutex::new(echo)),
            }
        }
    }

    enum EchoMessage {
        Echo(String),
        Stop,
    }

    impl Actor<EchoMessage> for EchoPeriodicWorker {
        async fn start(&mut self) -> VoidRes {
            self.delegate._start().await
        }

        async fn stop(&mut self) -> VoidRes {
            self.delegate._stop().await
        }

        async fn process(&mut self, _message: EchoMessage) -> VoidRes {
            log::info!("EchoPeriodicWorker received a message");
            match _message {
                EchoMessage::Echo(msg) => {
                    let mut echo = self.echo.lock()?;
                    *echo = msg;
                    log::info!("EchoPeriodicWorker echo updated to: {}", *echo);
                }
                EchoMessage::Stop => {
                    self.stop().await?;
                }
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn smoke_test() -> VoidRes {
        init_logger();
        let worker =
            EchoPeriodicWorker::new(Duration::from_secs(1), "Initial Echo Message".to_string());

        let handler = spawn_actor_with(worker, None).await?;

        tokio::time::sleep(Duration::from_secs(1)).await;
        handler
            .send(EchoMessage::Echo("New Echo Message".to_string()))
            .await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        handler.send(EchoMessage::Stop).await?;

        Ok(())
    }
}
