#[cfg(test)]
mod tests;

use crate::VoidRes;
use crate::actors::{Actor, ActorHandle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::task::JoinHandle;

struct ProcessWorker {
    name: String,
    exe: String,
    cmd: String,
    env: Vec<(String, String)>,
    handle: Option<JoinHandle<()>>,
    child_abort: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ProcessWorker {
    pub fn new<T: Into<String>>(name: T, exe: T, cmd: T, env: Vec<(T, T)>) -> Self {
        Self {
            name: name.into(),
            exe: exe.into(),
            cmd: cmd.into(),
            env: env.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
            handle: None,
            child_abort: None,
        }
    }
}

enum ProcessWorkerMessage {
    Stop,
}

impl Actor<ProcessWorkerMessage> for ProcessWorker {
    async fn start(&mut self) -> VoidRes {
        let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel();
        self.child_abort = Some(abort_tx);
 
        let exe = self.exe.clone();
        let cmd = self.cmd.clone();
        let env = self.env.clone();
        let name = self.name.clone();

        let handle = tokio::spawn(async move {
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
        });

        self.handle = Some(handle);
        Ok(())
    }

    async fn stop(&mut self) -> VoidRes { 
        if let Some(abort_tx) = self.child_abort.take() {
            let _ = abort_tx.send(());
        }

        if let Some(handle) = self.handle.take() {
            if let Err(e) = handle.await {
                log::error!(target:&self.name,"Error joining the process task: {}", e);
            }
        }

        Ok(())
    }

    async fn process(&mut self, message: ProcessWorkerMessage) -> VoidRes {
        match message {
            ProcessWorkerMessage::Stop => self.stop().await,
        }
    }
}
