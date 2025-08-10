pub mod error;
pub mod handler;
mod tests;

use crate::error::{SshError, SshResult, SshResultVoid};
use crate::handler::{BaseSshHandler, SshHandler};
use actix::{
    Actor, ActorContext, AsyncContext, Context, Handler, Message, SpawnHandle, WrapFuture,
};
use actor::{ActorResult, ActorResultVoid, ActorServiceMessage};
use russh::client::Handle;
use russh::server::Config;
use russh_keys::key::KeyPair;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

type OsFiles = Arc<Mutex<HashMap<String, Vec<u8>>>>;
type CmdProcessor =
    Box<dyn Fn(&str, OsFiles) -> Option<SshResult<(String, OsFiles)>> + Send + Sync>;

pub struct SshServer {
    key: String,
    host: String,
    port: u16,
    files: OsFiles,
    command_history: Arc<Mutex<Vec<String>>>,
    cmd_handler: BaseSshHandler,
}

impl Default for SshServer {
    fn default() -> Self {
        SshServer::new("ssh_server", "127.0.0.1", 2222, None)
    }
}

impl SshServer {
    pub fn new(
        key: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        cmd_processors: Option<Vec<CmdProcessor>>,
    ) -> Self {
        SshServer {
            key: key.into(),
            host: host.into(),
            port,
            files: Arc::new(Mutex::new(HashMap::new())),
            command_history: Arc::new(Mutex::new(Vec::new())),
            cmd_handler: cmd_processors.into(),
        }
    }
}

impl Actor for SshServer {
    type Context = Context<Self>;
}

impl Handler<ActorServiceMessage> for SshServer {
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ActorServiceMessage::Start => {
                let mut config = Config::default();
                config.keys.push(KeyPair::generate_ed25519().unwrap());
                config.auth_rejection_time = std::time::Duration::from_secs(1);

                let config = Arc::new(config);
                let addr = format!("{}:{}", self.host, self.port);

                log::info!("Starting SSH server {} on {}", self.key, addr);
                let files = self.files.clone();
                let command_history = self.command_history.clone();
                let cmd_handler = self.cmd_handler.clone();
                ctx.spawn(
                    async move {
                        let server_config = config.clone();
                        let listener = match tokio::net::TcpListener::bind(&addr).await {
                            Ok(l) => l,
                            Err(e) => {
                                log::error!("Failed to bind SSH server to {}: {}", addr, e);
                                return;
                            }
                        };
                        log::info!("SSH server listening on {}", addr);

                        loop {
                            match listener.accept().await {
                                Ok((socket, peer_addr)) => {
                                    log::info!("New SSH connection from {}", peer_addr);

                                    let handler = SshHandler::new(
                                        files.clone(),
                                        command_history.clone(),
                                        cmd_handler.clone(),
                                    );

                                    let conn_config = server_config.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) =
                                            russh::server::run_stream(conn_config, socket, handler)
                                                .await
                                        {
                                            log::error!("SSH connection error: {:?}", e);
                                        }
                                    });
                                }
                                Err(e) => {
                                    log::error!("Failed to accept SSH connection: {}", e);
                                    sleep(Duration::from_millis(100)).await;
                                }
                            }
                        }
                    }
                    .into_actor(self),
                );
            }
            ActorServiceMessage::Stop => {
                ctx.stop();
                log::info!("SSH server stopped");
            }
        }

        Ok(())
    }
}

impl Handler<SshFileOperation> for SshServer {
    type Result = SshResultVoid;

    fn handle(&mut self, msg: SshFileOperation, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SshFileOperation::Add(path, content) => {
                log::info!("Add file {}", path);
                self.files.lock()?.insert(path, content);
            }
            SshFileOperation::Remove(path) => {
                log::info!("Remove file {}", path);
                self.files.lock()?.remove(&path);
            }
        }

        Ok(())
    }
}

impl Handler<AddProcessor> for SshServer {
    type Result = SshResultVoid;

    fn handle(&mut self, msg: AddProcessor, ctx: &mut Self::Context) -> Self::Result {
        log::info!("Add processor");
        self.cmd_handler.add_processor(msg.0)?;
        Ok(())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "SshResultVoid")]
pub enum SshFileOperation {
    Add(String, Vec<u8>),
    Remove(String),
}
#[derive(Message)]
#[rtype(result = "SshResultVoid")]
pub struct AddProcessor(CmdProcessor);
