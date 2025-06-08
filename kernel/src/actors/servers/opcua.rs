pub mod data;
mod macros;
#[cfg(test)]
mod tests;

use crate::actors::Actor;
use crate::actors::servers::ServerError;
use crate::actors::servers::opcua::data::ServerStructure;
use crate::{Res, VoidRes};
use opcua::crypto::SecurityPolicy;
use opcua::server::builder::ServerBuilder;
use opcua::server::prelude::{AddressSpace, ServerEndpoint};
use opcua::server::server::Server as InnerServer;
use opcua::sync::RwLock;
use opcua::types::{DateTime, MessageSecurityMode, NodeId, Variant};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct OpcuaServer {
    id: String,
    host: String,
    port: u16,
    server_handle: Option<JoinHandle<()>>,
    structure: ServerStructure,
    server: Arc<RwLock<InnerServer>>,
}

impl OpcuaServer {
    pub fn new_default(structure: Option<ServerStructure>) -> Self {
        OpcuaServer::new(
            "opcua_server",
            "127.0.0.1",
            4840,
            structure.unwrap_or_default(),
        )
        .expect("Failed to create default OPC UA server")
    }

    pub fn new(
        id: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        structure: ServerStructure,
    ) -> Res<Self> {
        let id = id.into();
        let host = host.into();
        let server = Arc::new(RwLock::new(try_to_build_server(
            id.as_str(),
            host.as_str(),
            port as usize,
        )?));
        structure.process_server(server.clone())?;
        Ok(OpcuaServer {
            id,
            host,
            port,
            structure,
            server,
            server_handle: None,
        })
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }
}
fn try_to_build_server(id: &str, host: &str, port: usize) -> Res<InnerServer> {
    ServerBuilder::new()
        .application_name(id)
        .discovery_urls(vec![format!("opc.tcp://{}:{}", host, port)])
        .create_sample_keypair(true)
        .pki_dir("./pki-server")
        .discovery_server_url(None)
        .trust_client_certs()
        .endpoint(
            "none",
            ServerEndpoint::new(
                "/",
                SecurityPolicy::None,
                MessageSecurityMode::None,
                &["ANONYMOUS".to_string()],
            ),
        )
        .server()
        .ok_or(ServerError::StartError("Failed to create server".into(), id.to_string()).into())
}
impl Actor<OpcuaMessage> for OpcuaServer {
    async fn start(&mut self) -> VoidRes {
        log::info!(
            "Starting OPC UA server {} on {}:{}",
            self.id(),
            self.host,
            self.port
        );

        self.server_handle = Some(tokio::spawn(InnerServer::new_server_task(
            self.server.clone(),
        )));
        Ok(())
    }

    async fn stop(&mut self) -> VoidRes {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            log::info!("OPC UA server stopped");
        }
        Ok(())
    }

    async fn process(&mut self, message: OpcuaMessage) -> VoidRes {
        match message {
            OpcuaMessage::Start => {
                self.start().await?;
                Ok(())
            }
            OpcuaMessage::Stop => {
                self.stop().await?;
                Ok(())
            }
            OpcuaMessage::UpdateValue { node_id, value } => {
                let serv = self.server.write();
                let addr_space_ref = serv.address_space();
                let mut space = addr_space_ref.write();
                let res = space.set_variable_value(
                    node_id.clone(),
                    value,
                    &DateTime::now(),
                    &DateTime::now(),
                );
                if res {
                    Ok(())
                } else {
                    Err(ServerError::RuntimeError("Failed to update value".into()).into())
                }
            }
        }
    }
}

pub enum OpcuaMessage {
    Start,
    Stop,
    UpdateValue { node_id: NodeId, value: Variant },
}
