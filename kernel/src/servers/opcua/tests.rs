use crate::servers::opcua::data::{Namespace, Node, ServerStructure};
use crate::servers::opcua::{OpcuaMessage, OpcuaServer};
use crate::servers::spawn_server;
use crate::{VoidRes, folder, init_logger, object, variable};
use opcua::types::{DataTypeId, NodeId, QualifiedName, Variant};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn smoke_opcua() -> VoidRes {
    init_logger();
    let folder = folder!(1, 11, "Folder", "Folder" ;
            folder!(1, 2, "SubFolder", "SubFolder" ;
                object!(1, 3, "Object", "Object" ;
                    variable!(1, 4, "Variable", "Variable", DataTypeId::UInt16, 1, Variant::UInt16(1))
                )
        )
    );
    let server = OpcuaServer::new_default(Some(ServerStructure::new(
        vec![Namespace(1, "namespace1".to_string())],
        vec![folder],
    )));
    let server_handle = spawn_server(server, None)?;
    sleep(Duration::from_millis(1000000)).await;

    server_handle.send(OpcuaMessage::Stop).await?;
    sleep(Duration::from_millis(100)).await;

    Ok(())
}
