use crate::data::ServerStructure;
use actix::{Actor, ActorContext, AsyncContext, Context, Handler, Message, WrapFuture};
use actor::{ActorError, ActorResult, ActorResultVoid, ActorServiceMessage};
use opcua::client::prelude::Config;
use opcua::server::config::ServerConfig;
use opcua::server::prelude::Server;
use opcua::server::server::Server as InnerServer;
use opcua::sync::RwLock;
use opcua::types::{DateTime, NodeId, Variant};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub mod data;
pub mod macros;

pub struct OpcuaServer {
    key: String,
    server: Arc<RwLock<InnerServer>>,
}

impl OpcuaServer {
    pub fn new(
        id: impl Into<String>,
        cfg: &PathBuf,
        structure: ServerStructure,
    ) -> ActorResult<Self> {
        let key = id.into();
        let server = Arc::new(RwLock::new(try_to_build_server(key.as_str(), cfg)?));
        structure.process_server(server.clone())?;
        Ok(OpcuaServer { key, server })
    }
}
fn try_to_build_server(id: &str, config: &Path) -> ActorResult<InnerServer> {
    ServerConfig::load(config)
        .map(Server::new)
        .map_err(|_| ActorError::StartupError(format!("Failed to load server config {id}")))
}

impl Actor for OpcuaServer {
    type Context = Context<Self>;
}

impl Handler<ActorServiceMessage> for OpcuaServer {
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: ActorServiceMessage, ctx: &mut Self::Context) -> ActorResultVoid {
        match msg {
            ActorServiceMessage::Start => {
                log::info!("Starting server {}", self.key);
                let serv = self.server.clone();
                ctx.spawn(
                    async move {
                        InnerServer::new_server_task(serv).await;
                    }
                    .into_actor(self),
                );
            }
            ActorServiceMessage::Stop => {
                log::info!("Stopping server {}", self.key);
                ctx.stop();
            }
        }

        Ok(())
    }
}

impl Handler<UpdateValueMessage> for OpcuaServer {
    type Result = ActorResultVoid;

    fn handle(&mut self, msg: UpdateValueMessage, _ctx: &mut Self::Context) -> Self::Result {
        let UpdateValueMessage { node_id, value } = msg;
        let serv = self.server.write();
        let addr_space_ref = serv.address_space();
        let mut space = addr_space_ref.write();
        let res =
            space.set_variable_value(node_id.clone(), value, &DateTime::now(), &DateTime::now());
        if res {
            Ok(())
        } else {
            Err(ActorError::RuntimeError(
                "Failed to update the value".into(),
            ))
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "ActorResultVoid")]
struct UpdateValueMessage {
    node_id: NodeId,
    value: Variant,
}
