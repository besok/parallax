use actix::{Actor, ActorContext, AsyncContext, Context, Handler, SpawnHandle, WrapFuture};
use actor::{ActorResultVoid, ActorServiceMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;
use utils::logger_on;

pub struct ProcessActor {
    name: String,
    exe: String,
    cmd: String,
    env: Vec<(String, String)>,
    child_abort: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ProcessActor {
    pub fn new<T: Into<String>>(name: T, exe: T, cmd: T, env: Vec<(T, T)>) -> Self {
        Self {
            name: name.into(),
            exe: exe.into(),
            cmd: cmd.into(),
            env: env.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
            child_abort: None,
        }
    }
}

impl Actor for ProcessActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let exe = self.exe.clone();
        let cmd = self.cmd.clone();
        let env = self.env.clone();
        let name = self.name.clone();
        let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel();
        self.child_abort = Some(abort_tx);
        let handle = ctx.spawn(
            async move {
                let mut child = Command::new(exe)
                    .arg(cmd)
                    .envs(env)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                    .expect("Failed to spawn the process");


                let stdout = child.stdout.take().expect("Failed to capture stdout");
                let stderr = child.stderr.take().expect("Failed to capture stderr");

                let mut stdout_reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                loop {
                    tokio::select! {
                                    Ok(Some(line)) = stdout_reader.next_line() => {
                                        log::info!(target:&name, "Output: {}", line);
                                    }
                                    Ok(Some(line)) = stderr_reader.next_line() => {
                                        log::error!(target:&name,"Error: {}", line);
                                    }
                                    status = child.wait() => {
                                        log::info!(target:&name,"The process exited with status: {:?}", status);
                                        break;
                                    }
                                    _ = &mut abort_rx => {
                                        log::info!(target:&name,"Received abort signal, killing Python process...");
                                        let _ = child.kill().await;
                                        break;
                                    }
                                }
                }
            }
                .into_actor(self),
        );
    }
}
impl Handler<ActorServiceMessage> for ProcessActor {
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ActorServiceMessage::Stop => {
                if let Some(abort_tx) = self.child_abort.take() {
                    let _ = abort_tx.send(());
                }
                ctx.stop();
                log::info!("SSH server stopped");
            }
            ActorServiceMessage::Start => {
                log::info!("Process actor started");
            }
        }

        Ok(())
    }
}

#[actix::test]
async fn test_process() -> ActorResultVoid {
    logger_on();

    let actor = ProcessActor::new("Test", "python3", "--version", vec![]).start();

    tokio::time::sleep(Duration::from_secs(1)).await;

    actor.send(ActorServiceMessage::Stop);
    Ok(())
}
