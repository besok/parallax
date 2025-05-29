use crate::actors::servers::ssh::{CmdProcessor, OsFiles};
use crate::{Res, VoidRes};
use std::io;

use crate::actors::servers::ServerError;
use async_trait::async_trait;
use russh::server::{Auth, Msg, Session};
use russh::{Channel, ChannelId, server};
use russh_keys::key::PublicKey;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct BaseSshHandler {
    processors: Arc<Mutex<Vec<CmdProcessor>>>,
}

impl From<Option<Vec<CmdProcessor>>> for BaseSshHandler {
    fn from(processors: Option<Vec<CmdProcessor>>) -> Self {
        processors.map(BaseSshHandler::new).unwrap_or_default()
    }
}

impl Default for BaseSshHandler {
    fn default() -> Self {
        BaseSshHandler {
            processors: Arc::new(Mutex::new(default_cmd_processors())),
        }
    }
}

impl BaseSshHandler {
    pub fn add_processor(&mut self, processor: CmdProcessor) -> VoidRes {
        let mut guard = self.processors.lock()?;
        guard.insert(0, processor);
        Ok(())
    }
    pub fn new(processors: Vec<CmdProcessor>) -> Self {
        BaseSshHandler {
            processors: Arc::new(Mutex::new(processors)),
        }
    }
    fn handle_command(&self, cmd: &str, files: OsFiles) -> Res<(String, OsFiles)> {
        for processor in self.processors.lock()?.iter() {
            if let Some(res) = processor(cmd, files.clone()) {
                return res;
            }
        }
        Ok((format!("Unknown command: {}\n", cmd), files))
    }
}

pub fn default_cmd_processors() -> Vec<CmdProcessor> {
    vec![
        Box::new(|cmd, files| {
            if cmd.trim() == "ls" {
                let guard = files.lock();
                if let Err(e) = guard {
                    return Some(Err(e.into()));
                }

                let file_list = guard
                    .unwrap()
                    .keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                Some(Ok((
                    if file_list.is_empty() {
                        "No files found\n".to_string()
                    } else {
                        format!("{}\n", file_list)
                    },
                    files,
                )))
            } else {
                None
            }
        }),
        Box::new(|cmd, files| {
            if cmd.trim() == "ssh_test_server" {
                Some(Ok(("It is an Ssh test server!\n".to_string(), files)))
            } else {
                None
            }
        }),
    ]
}

pub struct SshHandler {
    files: OsFiles,
    command_history: Arc<Mutex<Vec<String>>>,
    cmd_handler: BaseSshHandler,
}

impl SshHandler {
    pub fn new(
        files: OsFiles,
        command_history: Arc<Mutex<Vec<String>>>,
        cmd_handler: BaseSshHandler,
    ) -> Self {
        Self {
            files,
            command_history,
            cmd_handler,
        }
    }
}

#[async_trait]
impl server::Handler for SshHandler {
    type Error = ServerError;

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        Ok((self, Auth::Accept))
    }

    async fn auth_publickey(
        self,
        _user: &str,
        _public_key: &PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        Ok((self, Auth::Accept))
    }

    async fn channel_open_session(
        self,
        _channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        Ok((self, true, session))
    }

    async fn exec_request(
        self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let cmd = String::from_utf8_lossy(data).to_string();

        if let Ok(mut history) = self.command_history.lock() {
            history.push(cmd.clone());
        }

        let result = match self.cmd_handler.handle_command(&cmd, self.files.clone()) {
            Ok((response, _)) => response,
            Err(e) => format!("Error: {:?}\n", e),
        };

        let _ = session.data(channel, result.as_bytes().to_vec().into());
        let _ = session.exit_status_request(channel, 0);
        let _ = session.close(channel);

        Ok((self, session))
    }
}
