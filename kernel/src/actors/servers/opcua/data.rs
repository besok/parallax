use crate::actors::servers::ServerError;
use crate::error::KernelError;
use crate::{Res, VoidRes};
use opcua::server::address_space::AddressSpace;
use opcua::server::prelude::{
    NodeBase, ObjectBuilder, ObjectTypeBuilder, VariableBuilder, VariableTypeBuilder,
};
use opcua::server::server::Server;
use opcua::sync::RwLock;
use opcua::types::{
    DataTypeId, LocalizedText, MethodAttributes, NodeId, QualifiedName, VariableAttributes, Variant,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLockWriteGuard;

pub type NamespaceIndex = u16;
pub type NamespaceUri = String;

#[derive(Debug, Clone)]
pub struct Namespace(pub NamespaceIndex, pub NamespaceUri);

#[derive(Debug, Clone)]
pub struct Node {
    pub node_id: NodeId,
    pub browse_name: QualifiedName,
    pub display_name: LocalizedText,
    pub delegate: NodeDelegate,
    pub children: Vec<Node>,
}

impl Node {
    fn process(node: Node, parent: Option<&NodeId>, addr: &mut AddressSpace) -> VoidRes {
        let id = &node.node_id;
        log::info!("Processing node: {:?}", id);
        let browse_name = node.browse_name.clone();
        let display_name = node.display_name.clone();
        match &node.delegate {
            NodeDelegate::Folder => {
                if !addr.add_folder_with_id(
                    id,
                    browse_name,
                    display_name,
                    parent.unwrap_or(&NodeId::root_folder_id()),
                ) {
                    return Err(KernelError::client(&format!(
                        "Failed to insert folder: {}",
                        id
                    )));
                }
            }
            NodeDelegate::Object => {
                let object = ObjectBuilder::new(id, browse_name, display_name)
                    .organized_by(parent.unwrap_or(&NodeId::objects_folder_id()));

                if !object.is_valid() {
                    return Err(KernelError::client(&format!("Invalid object type: {}", id)));
                }

                if !object.insert(addr) {
                    return Err(KernelError::client(&format!(
                        "Failed to insert object: {}",
                        id
                    )));
                }
            }

            NodeDelegate::Variable {
                value,
                data_type,
                value_rank,
            } => {
                let builder = VariableBuilder::new(id, browse_name, display_name)
                    .value(value.clone())
                    .data_type(data_type)
                    .value_rank(*value_rank)
                    .organized_by(parent.unwrap_or(&NodeId::root_folder_id()).clone());

                if !&builder.is_valid() {
                    return Err(KernelError::client(&format!(
                        "Invalid variable type: {}",
                        id
                    )));
                }
                if !builder.insert(addr) {
                    return Err(KernelError::client(&format!(
                        "Failed to insert variable type: {}",
                        id
                    )));
                }
            }
            NodeDelegate::Property {
                value,
                data_type,
                value_rank,
            } => {
                let builder =
                    VariableBuilder::new(id, node.browse_name.clone(), node.display_name.clone())
                        .property_of(parent.unwrap_or(&NodeId::root_folder_id()))
                        .value(value.clone())
                        .data_type(data_type)
                        .value_rank(*value_rank);

                if !&builder.is_valid() {
                    return Err(KernelError::client(&format!(
                        "Invalid property type: {}",
                        id
                    )));
                }
                if !builder.insert(addr) {
                    return Err(KernelError::client(&format!(
                        "Failed to insert property type: {}",
                        id
                    )));
                }
            }

            NodeDelegate::Reference {
                target,
                reference_type,
            } => {
                addr.insert_reference(id, target, reference_type);
            }
        }

        for child in node.children {
            Node::process(child, Some(id), addr)?;
        }

        Ok(())
    }

    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn folder(
        node_id: NodeId,
        browse_name: QualifiedName,
        display_name: LocalizedText,
        children: Vec<Node>,
    ) -> Self {
        Node {
            node_id,
            browse_name,
            display_name,
            delegate: NodeDelegate::Folder,
            children,
        }
    }

    pub fn object(
        node_id: NodeId,
        browse_name: QualifiedName,
        display_name: LocalizedText,
        children: Vec<Node>,
    ) -> Self {
        Node {
            node_id,
            browse_name,
            display_name,
            delegate: NodeDelegate::Object,
            children,
        }
    }

    pub fn variable(
        node_id: NodeId,
        browse_name: QualifiedName,
        display_name: LocalizedText,
        data_type: DataTypeId,
        value_rank: i32,
        value: Variant,
    ) -> Self {
        Node {
            node_id,
            browse_name,
            display_name,
            delegate: NodeDelegate::Variable {
                value,
                data_type,
                value_rank,
            },
            children: Vec::new(),
        }
    }

    pub fn property(
        node_id: NodeId,
        browse_name: QualifiedName,
        display_name: LocalizedText,
        data_type: DataTypeId,
        value_rank: i32,
        value: Variant,
    ) -> Self {
        Node {
            node_id,
            browse_name,
            display_name,
            delegate: NodeDelegate::Property {
                value,
                data_type,
                value_rank,
            },
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodeDelegate {
    Folder,
    Object,
    Variable {
        value: Variant,
        data_type: DataTypeId,
        value_rank: i32,
    },
    Property {
        value: Variant,
        data_type: DataTypeId,
        value_rank: i32,
    },
    Reference {
        target: NodeId,
        reference_type: NodeId,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ServerStructure {
    pub namespaces: Vec<Namespace>,
    pub nodes: Vec<Node>,
}

impl ServerStructure {
    pub fn new(namespaces: Vec<Namespace>, nodes: Vec<Node>) -> Self {
        ServerStructure { namespaces, nodes }
    }

    pub fn process_server(&self, server: Arc<RwLock<Server>>) -> VoidRes {
        for namespace in &self.namespaces {
            if namespace.0 == 0 {
                return Err(
                    ServerError::ClientError("Namespace index cannot be 0".to_string()).into(),
                );
            }
            if namespace.1.is_empty() {
                return Err(
                    ServerError::ClientError("Namespace URI  cannot be empty".to_string()).into(),
                );
            }
        }

        let serv = server.write();

        let addr_ref = serv.address_space();
        let mut addr = addr_ref.write();

        let mut sorted = self.namespaces.clone();
        sorted.sort_by_key(|Namespace(idx, _)| *idx);

        for Namespace(idx, uri) in sorted.iter() {
            let result = addr
                .register_namespace(uri)
                .map_err(|_| KernelError::client("Can not register namespace"))?;

            if *idx != result {
                return Err(KernelError::client(
                    format!("Namespace index mismatch {idx} != {result}").as_str(),
                ));
            }
        }

        for node in &self.nodes {
            Node::process(node.clone(), None, &mut addr)?;
        }

        Ok(())
    }
}
