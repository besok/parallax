// pub mod handler;
// #[cfg(test)]
// mod tests;
//
// use crate::actors::Actor;
// use crate::actors::servers::ServerError;
// use crate::actors::servers::ssh::handler::{BaseSshHandler, SshHandler, default_cmd_processors};
// use crate::{Res, VoidRes};
// use russh::server::{Config, Handler};
// use russh_keys::key::KeyPair;
// use std::collections::HashMap;
// use std::sync::{Arc, Mutex};
// use std::time::Duration;
// use tokio::task::JoinHandle;
// use tokio::time::sleep;
//
// type OsFiles = Arc<Mutex<HashMap<String, Vec<u8>>>>;
// type CmdProcessor = Box<dyn Fn(&str, OsFiles) -> Option<Res<(String, OsFiles)>> + Send + Sync>;
//
// pub struct SshServer {
//     id: String,
//     host: String,
//     port: u16,
//     server_handle: Option<JoinHandle<()>>,
//     files: OsFiles,
//     command_history: Arc<Mutex<Vec<String>>>,
//     cmd_handler: BaseSshHandler,
// }
//
// impl Default for SshServer {
//     fn default() -> Self {
//         SshServer::new("ssh_server", "127.0.0.1", 2222, None)
//     }
// }
//
// impl SshServer {
//     pub fn new(
//         id: impl Into<String>,
//         host: impl Into<String>,
//         port: u16,
//         cmd_processors: Option<Vec<CmdProcessor>>,
//     ) -> Self {
//         SshServer {
//             id: id.into(),
//             host: host.into(),
//             port,
//             server_handle: None,
//             files: Arc::new(Mutex::new(HashMap::new())),
//             command_history: Arc::new(Mutex::new(Vec::new())),
//             cmd_handler: cmd_processors.into(),
//         }
//     }
// }
//
// impl Actor<SshMessage> for SshServer {
//     fn id(&self) -> String {
//         self.id.clone()
//     }
//
//     async fn start(&mut self) -> VoidRes {
//         let mut config = Config::default();
//         config.keys.push(KeyPair::generate_ed25519().unwrap());
//         config.auth_rejection_time = std::time::Duration::from_secs(1);
//
//         let config = Arc::new(config);
//         let addr = format!("{}:{}", self.host, self.port);
//
//         log::info!("Starting SSH server {} on {}", self.id(), addr);
//         let files = self.files.clone();
//         let command_history = self.command_history.clone();
//         let cmd_handler = self.cmd_handler.clone();
//
//         self.server_handle = Some(tokio::spawn(async move {
//             let server_config = config.clone();
//             let listener = match tokio::net::TcpListener::bind(&addr).await {
//                 Ok(l) => l,
//                 Err(e) => {
//                     log::error!("Failed to bind SSH server to {}: {}", addr, e);
//                     return;
//                 }
//             };
//             log::info!("SSH server listening on {}", addr);
//
//             loop {
//                 match listener.accept().await {
//                     Ok((socket, peer_addr)) => {
//                         log::info!("New SSH connection from {}", peer_addr);
//
//                         let handler = SshHandler::new(
//                             files.clone(),
//                             command_history.clone(),
//                             cmd_handler.clone(),
//                         );
//
//                         let conn_config = server_config.clone();
//                         tokio::spawn(async move {
//                             if let Err(e) =
//                                 russh::server::run_stream(conn_config, socket, handler).await
//                             {
//                                 log::error!("SSH connection error: {:?}", e);
//                             }
//                         });
//                     }
//                     Err(e) => {
//                         log::error!("Failed to accept SSH connection: {}", e);
//                         sleep(Duration::from_millis(100)).await;
//                     }
//                 }
//             }
//         }));
//
//         Ok(())
//     }
//
//     async fn stop(&mut self) -> VoidRes {
//         if let Some(handle) = self.server_handle.take() {
//             handle.abort();
//             log::info!("SSH server stopped");
//         }
//         Ok(())
//     }
//
//     async fn process(&mut self, message: SshMessage) -> VoidRes {
//         match message {
//             SshMessage::Start => self.start().await?,
//             SshMessage::Stop => self.stop().await?,
//             SshMessage::AddFile { path, content } => {
//                 self.files.lock()?.insert(path, content);
//             }
//             SshMessage::RemoveFile { path } => {
//                 self.files.lock()?.remove(&path);
//             }
//             SshMessage::AddProcessor(processor) => {
//                 self.cmd_handler.add_processor(processor)?;
//             }
//         }
//         Ok(())
//     }
// }
//
// pub enum SshMessage {
//     Start,
//     Stop,
//     AddFile { path: String, content: Vec<u8> },
//     RemoveFile { path: String },
//     AddProcessor(CmdProcessor),
// }
