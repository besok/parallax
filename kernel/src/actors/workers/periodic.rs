use crate::VoidRes;
use crate::actors::Actor;
use crate::error::KernelError;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
struct PeriodicWorkerParams {
    stop_signal: Option<Sender<()>>,
    interval: Duration,
}

impl PeriodicWorkerParams {
    pub fn new(interval: Duration) -> Self {
        Self {
            stop_signal: None,
            interval,
        }
    }
}

trait PeriodicWorker<Mes>: Clone + Send + 'static {
    fn task(&mut self) -> VoidRes;

    fn error(&self, error: KernelError) -> VoidRes {
        log::error!("Periodic worker error: {:?}", error);
        Ok(())
    }

    fn _process(&mut self, _message: Mes) -> VoidRes {
        Ok(())
    }

    fn _stop(&self) -> VoidRes {
        if let Some(tx) = &self.params().stop_signal {
            let _ = tx.try_send(());
        }
        Ok(())
    }

    fn params(&self) -> &PeriodicWorkerParams;
    fn params_mut(&mut self) -> &mut PeriodicWorkerParams;

    fn _start(&mut self) -> VoidRes {
        let interval = self.params().interval;
        let mut interval_timer = tokio::time::interval(interval);
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);
        self.params_mut().stop_signal = Some(stop_tx);

        let mut cloned_self = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        if let Err(e) = cloned_self.task() {
                            if let Err(e) = cloned_self.error(e) {
                                log::error!("The report error returns a failed state: {:?}", e);
                                break;
                            }
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
}

impl<Mes, PW> Actor<Mes> for PW
where
    PW: PeriodicWorker<Mes>,
{
    fn start(&mut self) -> VoidRes {
        self._start()
    }

    fn stop(&mut self) -> VoidRes {
        self._stop()
    }

    fn process(&mut self, message: Mes) -> VoidRes {
        self._process(message)
    }
}

#[cfg(test)]
mod tests {
    use crate::actors::spawn_actor;
    use crate::actors::workers::periodic::{PeriodicWorker, PeriodicWorkerParams};
    use crate::{VoidRes, init_logger};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::mpsc::Sender;

    #[derive(Clone)]
    struct EchoPeriodicWorker {
        base: PeriodicWorkerParams,
        echo: Arc<Mutex<String>>,
    }

    enum EchoMessage {
        Echo(String),
        StopEchoServer,
    }

    impl PeriodicWorker<EchoMessage> for EchoPeriodicWorker {
        fn task(&mut self) -> VoidRes {
            log::info!("EchoPeriodicWorker is doing work: {:?}", *self.echo);
            Ok(())
        }

        fn _process(&mut self, _message: EchoMessage) -> VoidRes {
            log::info!("EchoPeriodicWorker received a message");
            match _message {
                EchoMessage::Echo(msg) => {
                    let mut echo = self.echo.lock()?;
                    *echo = msg;
                    log::info!("EchoPeriodicWorker echo updated to: {}", *echo);
                }
                EchoMessage::StopEchoServer => {
                    self._stop()?;
                }
            }
            Ok(())
        }

        fn params(&self) -> &PeriodicWorkerParams {
            &self.base
        }

        fn params_mut(&mut self) -> &mut PeriodicWorkerParams {
            &mut self.base
        }
    }

    #[tokio::test]
    async fn smoke_test() -> VoidRes {
        init_logger();
        let worker = EchoPeriodicWorker {
            base: PeriodicWorkerParams::new(Duration::from_secs(2)),
            echo: Arc::new(Mutex::new("Initial Echo Message".to_string())),
        };

        let handler = spawn_actor(worker, None)?;

        tokio::time::sleep(Duration::from_secs(1)).await;
        handler
            .send(EchoMessage::Echo("New Echo Message".to_string()))
            .await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        handler.send(EchoMessage::StopEchoServer).await?;

        Ok(())
    }
}
