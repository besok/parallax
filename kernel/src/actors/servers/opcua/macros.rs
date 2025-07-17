#[macro_export]
macro_rules! folder {
    // Basic folder with namespace index, id, browse name, display name and children
    ($ns:expr, $id:expr, $browse_ns:expr, $browse_name:expr, $display_name:expr ; $($child:expr),* $(,)?) => {
        Node::folder(
            NodeId::new($ns, $id),
            QualifiedName::new($browse_ns, $browse_name),
            $display_name.into(),
            vec![$($child),*]
        )
    };

    // Shorthand version where browse_ns equals ns with children
    ($ns:expr, $id:expr, $browse_name:expr, $display_name:expr ; $($child:expr),* $(,)?) => {
        folder!($ns, $id, $ns, $browse_name, $display_name ; $($child),*)
    };

    // Basic folder without children
    ($ns:expr, $id:expr, $browse_ns:expr, $browse_name:expr, $display_name:expr) => {
        Node::folder(
            NodeId::new($ns, $id),
            QualifiedName::new($browse_ns, $browse_name),
            $display_name.into(),
            vec![]
        )
    };

    // Shorthand version without children
    ($ns:expr, $id:expr, $browse_name:expr, $display_name:expr) => {
        folder!($ns, $id, $ns, $browse_name, $display_name)
    };
}

#[macro_export]
macro_rules! object {
    // Object with namespace index, id, browse name, display name and children
    ($ns:expr, $id:expr, $browse_ns:expr, $browse_name:expr, $display_name:expr ; $($child:expr),* $(,)?) => {
        Node::object(
            NodeId::new($ns, $id),
            QualifiedName::new($browse_ns, $browse_name),
            $display_name.into(),
            vec![$($child),*]
        )
    };
    
    ($ns:expr, $id:expr, $browse_ns:expr, $browse_name:expr, $display_name:expr ; [] $children:expr) => {
        Node::object(
            NodeId::new($ns, $id),
            QualifiedName::new($browse_ns, $browse_name),
            $display_name.into(),
            $children
        )
    };
 
    // Shorthand version where browse_ns equals ns with children
    ($ns:expr, $id:expr, $browse_name:expr, $display_name:expr ; $($child:expr),* $(,)?) => {
        object!($ns, $id, $ns, $browse_name, $display_name ; $($child),*)
    };
    ($ns:expr, $id:expr, $browse_name:expr, $display_name:expr ; [] $children:expr) => {
        object!($ns, $id, $ns, $browse_name, $display_name ; [] $children)
    };
 
    // Object without children
    ($ns:expr, $id:expr, $browse_ns:expr, $browse_name:expr, $display_name:expr) => {
        Node::object(
            NodeId::new($ns, $id),
            QualifiedName::new($browse_ns, $browse_name),
            $display_name.into(),
            vec![]
        )
    };

    // Shorthand version without children
    ($ns:expr, $id:expr, $browse_name:expr, $display_name:expr) => {
        object!($ns, $id, $ns, $browse_name, $display_name)
    };
}

#[macro_export]
macro_rules! variable {
    // Variable with namespace index, id, browse name, display name, data type, rank and value
    ($ns:expr, $id:expr, $browse_ns:expr, $browse_name:expr, $display_name:expr, $data_type:expr, $value_rank:expr, $value:expr) => {
        Node::variable(
            NodeId::new($ns, $id),
            QualifiedName::new($browse_ns, $browse_name),
            $display_name.into(),
            $data_type,
            $value_rank,
            $value,
        )
    };
   
    // Shorthand version where browse_ns equals ns
    ($ns:expr, $id:expr, $browse_name:expr, $display_name:expr, $data_type:expr, $value_rank:expr, $value:expr) => {
        variable!(
            $ns,
            $id,
            $ns,
            $browse_name,
            $display_name,
            $data_type,
            $value_rank,
            $value
        )
    };
    
    ([] $ns:expr, $id:expr, $browse_name:expr, $display_name:expr, $data_type:expr, $value_rank:expr, $value:expr ; $($child:expr),* $(,)?) => {
    
        Node::variable_with(
            NodeId::new($ns, $id),
            QualifiedName::new($ns, $browse_name),
            $display_name.into(),
            $data_type,
            $value_rank, 
            $value,
            vec![$($child),*]
        )
   
    };

  
}
