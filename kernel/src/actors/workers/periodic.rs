use crate::VoidRes;
use crate::actors::Actor;
use crate::error::KernelError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

trait PeriodicWorker<Mes>: Send + Clone + 'static {
    fn interval(&self) -> Duration;

    fn do_work(&mut self) -> VoidRes;

    fn report_error(&self, error: KernelError) -> VoidRes {
        log::error!("Periodic worker error: {:?}", error);
        Ok(())
    }

    fn process_mes(&mut self, _message: Mes) -> VoidRes {
        Ok(())
    }

    fn set_stop_signal(&mut self, tx: Sender<()>);
    fn get_stop_signal(&self) -> Option<&Sender<()>>;
    fn stop_worker(&self) -> VoidRes {
        if let Some(tx) = self.get_stop_signal() {
            let _ = tx.try_send(());
        }
        Ok(())
    }
    fn start_worker(&mut self) -> VoidRes {
        let interval = self.interval();
        let mut interval_timer = tokio::time::interval(interval);
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel::<()>(1);
        self.set_stop_signal(stop_tx);

        let mut cloned_self = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        if let Err(e) = cloned_self.do_work() {
                            if let Err(e) = cloned_self.report_error(e) {
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
        self.start_worker()
    }

    fn stop(&mut self) -> VoidRes {
        self.stop_worker()
    }

    fn process(&mut self, message: Mes) -> VoidRes {
        self.process_mes(message)
    }
}

#[cfg(test)]
mod tests {
    use crate::actors::spawn_actor;
    use crate::actors::workers::periodic::PeriodicWorker;
    use crate::{VoidRes, init_logger};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::mpsc::Sender;

    #[derive(Clone)]
    struct EchoPeriodicWorker {
        interval: Duration,
        stop_signal: Option<Sender<()>>,
        echo: Arc<Mutex<String>>,
    }

    enum EchoMessage {
        Echo(String),
        StopEchoServer,
    }

    impl PeriodicWorker<EchoMessage> for EchoPeriodicWorker {
        fn interval(&self) -> Duration {
            self.interval
        }

        fn do_work(&mut self) -> VoidRes {
            log::info!("EchoPeriodicWorker is doing work: {:?}", *self.echo);
            Ok(())
        }

        fn process_mes(&mut self, _message: EchoMessage) -> VoidRes {
            log::info!("EchoPeriodicWorker received a message");
            match _message {
                EchoMessage::Echo(msg) => {
                    let mut echo = self.echo.lock()?;
                    *echo = msg;
                    log::info!("EchoPeriodicWorker echo updated to: {}", *echo);
                }
                EchoMessage::StopEchoServer => {
                    self.stop_worker()?;
                }
            }
            Ok(())
        }

        fn set_stop_signal(&mut self, tx: Sender<()>) {
            self.stop_signal = Some(tx);
        }

        fn get_stop_signal(&self) -> Option<&Sender<()>> {
            self.stop_signal.as_ref()
        }
    }

    #[tokio::test]
    async fn smoke_test() -> VoidRes {
        init_logger();
        let worker = EchoPeriodicWorker {
            interval: Duration::from_secs(1),
            stop_signal: None,
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
