// pub mod data;
// mod macros;
//
// use crate::actors::Actor;
// use crate::actors::servers::ServerError;
// use crate::actors::servers::opcua::data::{Node, ServerStructure};
// use crate::{Res, VoidRes};
// use opcua::crypto::SecurityPolicy;
// use opcua::server::builder::ServerBuilder;
// use opcua::server::config::ServerUserToken;
// use opcua::server::prelude::{AddressSpace, Config, Server, ServerConfig, ServerEndpoint};
// use opcua::server::server::Server as InnerServer;
// use opcua::sync::RwLock;
// use opcua::types::{DataTypeId, DateTime, MessageSecurityMode, NodeId, Variant};
// use std::path::PathBuf;
// use std::sync::Arc;
// use std::time::Duration;
// use tokio::sync::Mutex;
// use tokio::task::JoinHandle;
//
// pub struct OpcuaServer {
//     id: String,
//     server_handle: Option<JoinHandle<()>>,
//     structure: ServerStructure,
//     server: Arc<RwLock<InnerServer>>,
// }
//
// impl OpcuaServer {
//     pub fn new(id: impl Into<String>, cfg: &PathBuf, structure: ServerStructure) -> Res<Self> {
//         let id = id.into();
//         let server = Arc::new(RwLock::new(try_to_build_server(id.as_str(), cfg)?));
//         structure.process_server(server.clone())?;
//         Ok(OpcuaServer {
//             id,
//             structure,
//             server,
//             server_handle: None,
//         })
//     }
// }
// fn try_to_build_server(id: &str, config: &PathBuf) -> Res<InnerServer> {
//     Ok(ServerConfig::load(config)
//         .map(|cfg| Server::new(cfg))
//         .map_err(|_| {
//             ServerError::StartError("Failed to load server config".into(), id.to_string())
//         })?)
// }
// impl Actor<OpcuaMessage> for OpcuaServer {
//     fn id(&self) -> String {
//         self.id.clone()
//     }
//
//     async fn start(&mut self) -> VoidRes {
//         log::info!("Starting OPC UA server {}", self.id(),);
//
//         self.server_handle = Some(tokio::spawn(InnerServer::new_server_task(
//             self.server.clone(),
//         )));
//         Ok(())
//     }
//
//     async fn stop(&mut self) -> VoidRes {
//         if let Some(handle) = self.server_handle.take() {
//             handle.abort();
//             log::info!("OPC UA server stopped");
//         }
//         Ok(())
//     }
//
//     async fn process(&mut self, message: OpcuaMessage) -> VoidRes {
//         match message {
//             OpcuaMessage::Start => {
//                 self.start().await?;
//                 Ok(())
//             }
//             OpcuaMessage::Stop => {
//                 self.stop().await?;
//                 Ok(())
//             }
//             OpcuaMessage::UpdateValue { node_id, value } => {
//                 let serv = self.server.write();
//                 let addr_space_ref = serv.address_space();
//                 let mut space = addr_space_ref.write();
//                 let res = space.set_variable_value(
//                     node_id.clone(),
//                     value,
//                     &DateTime::now(),
//                     &DateTime::now(),
//                 );
//                 if res {
//                     Ok(())
//                 } else {
//                     Err(ServerError::RuntimeError("Failed to update value".into()).into())
//                 }
//             }
//         }
//     }
// }
//
// pub enum OpcuaMessage {
//     Start,
//     Stop,
//     UpdateValue { node_id: NodeId, value: Variant },
// }
