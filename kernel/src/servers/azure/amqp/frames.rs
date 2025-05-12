use crate::servers::azure::amqp::extract::extract_delivery_id;

pub(crate) fn create_sasl_mechanisms_frame() -> Vec<u8> {
    vec![
        0x00, 0x00, 0x00, 0x20, // Size (32 bytes)
        0x02, 0x01, 0x00, 0x00, // DOFF, Type (1=SASL), Channel
        0x00, 0x53, 0x40, 0xC0, // SASL-MECHANISMS performative
        0x03, 0xA3, 0x05, 0x50, // Array descriptor
        0x4C, 0x41, 0x49, 0x4E, // "PLAIN"
        0x00, 0x00, 0x00, 0x00, // padding
    ]
}

pub(crate) fn create_sasl_outcome_frame(success: bool) -> Vec<u8> {
    let code = if success { 0 } else { 1 }; // 0=OK, 1=AUTH
    vec![
        0x00, 0x00, 0x00, 0x10, // Size (16 bytes)
        0x02, 0x01, 0x00, 0x00, // DOFF, Type (1=SASL), Channel
        0x00, 0x53, 0x44, 0x43, // SASL-OUTCOME performative
        code, 0x00, 0x00, 0x00, // Outcome code
        0x00, 0x00, 0x00, 0x00, // padding
    ]
}

pub(crate) fn create_open_response() -> Vec<u8> {
    vec![
        0x00, 0x00, 0x00, 0x18, // Size (24 bytes)
        0x02, 0x00, 0x00, 0x00, // DOFF, Type (0=AMQP), Channel
        0x00, 0x53, 0x10, 0xD0, // Open performative
        0x00, 0x00, 0x00, 0x00, // Container ID (empty)
        0x00, 0x00, 0x00, 0x00, // Hostname (empty)
        0x00, 0x00, 0x27, 0x10, // Max frame size
    ]
}

pub(crate) fn create_begin_response() -> Vec<u8> {
    vec![
        0x00, 0x00, 0x00, 0x1C, // Size (28 bytes)
        0x02, 0x00, 0x00, 0x00, // DOFF, Type, Channel
        0x00, 0x53, 0x11, 0xC0, // Begin performative
        0x00, 0x00, 0x00, 0x00, // Remote Channel (0)
        0x00, 0x00, 0x00, 0x01, // Next outgoing ID
        0x00, 0x00, 0x00, 0x64, // Incoming window (100)
        0x00, 0x00, 0x00, 0x64, // Outgoing window (100)
    ]
}

pub(crate) fn create_attach_response(frame_data: &[u8]) -> Vec<u8> {
    vec![
        0x00, 0x00, 0x00, 0x20, // Size (32 bytes)
        0x02, 0x00, 0x00, 0x00, // DOFF, Type, Channel
        0x00, 0x53, 0x12, 0xC0, // Attach performative
        0x00, 0x00, 0x00, 0x00, // Name (empty string)
        0x00, 0x00, 0x00, 0x00, // Handle
        0x00, 0x01, 0x00, 0x00, // Role (0=sender, 1=receiver)
        0x00, 0x00, 0x00, 0x00, // Source
        0x00, 0x00, 0x00, 0x00, // Target
    ]
}

pub(crate) fn create_flow_frame(window_size: u32) -> Vec<u8> {
    // Simplified Flow frame
    let size_bytes = window_size.to_be_bytes();
    vec![
        0x00,
        0x00,
        0x00,
        0x1C, // Size (28 bytes)
        0x02,
        0x00,
        0x00,
        0x00, // DOFF, Type, Channel
        0x00,
        0x53,
        0x13,
        0xC0, // Flow performative
        0x00,
        0x00,
        0x00,
        0x00, // Next incoming ID
        size_bytes[0],
        size_bytes[1],
        size_bytes[2],
        size_bytes[3], // Incoming window
        0x00,
        0x00,
        0x00,
        0x01, // Next outgoing ID
        size_bytes[0],
        size_bytes[1],
        size_bytes[2],
        size_bytes[3], // Outgoing window
    ]
}

pub(crate) fn create_disposition_frame(frame_data: &[u8]) -> Vec<u8> {
    // Try to extract delivery ID from the transfer frame
    let delivery_id = extract_delivery_id(frame_data).unwrap_or(1);
    let id_bytes = delivery_id.to_be_bytes();

    // Simplified Disposition frame
    vec![
        0x00,
        0x00,
        0x00,
        0x18, // Size (24 bytes)
        0x02,
        0x00,
        0x00,
        0x00, // DOFF, Type, Channel
        0x00,
        0x53,
        0x15,
        0xC0, // Disposition performative
        0x01,
        0x00,
        0x00,
        0x00, // Role (1=receiver)
        id_bytes[0],
        id_bytes[1],
        id_bytes[2],
        id_bytes[3], // First delivery ID
        id_bytes[0],
        id_bytes[1],
        id_bytes[2],
        id_bytes[3], // Last delivery ID
        0x00,
        0x24,
        0x00,
        0x00, // Accepted outcome
    ]
}

pub(crate) fn create_transfer_frame(delivery_id: u32, message_data: &[u8]) -> Vec<u8> {
    // Calculate total frame size
    let total_size = 20 + message_data.len(); // Header + performative + data
    let size_bytes = (total_size as u32).to_be_bytes();
    let id_bytes = delivery_id.to_be_bytes();

    // Create frame
    let mut frame = Vec::with_capacity(total_size);

    // Frame header
    frame.extend_from_slice(&[
        size_bytes[0],
        size_bytes[1],
        size_bytes[2],
        size_bytes[3], // Size
        0x02,
        0x00,
        0x00,
        0x00, // DOFF, Type, Channel
    ]);

    // Transfer performative
    frame.extend_from_slice(&[
        0x00,
        0x53,
        0x14,
        0xC0, // Transfer performative
        id_bytes[0],
        id_bytes[1],
        id_bytes[2],
        id_bytes[3], // Delivery ID
        0x01,
        0x00,
        0x00,
        0x00, // Delivery tag + flag
    ]);

    // Message data
    frame.extend_from_slice(message_data);

    frame
}

// Helper functions for identifying AMQP frame types

pub(crate) fn is_sasl_init_frame(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // Look for SASL-INIT (0x41) performative
    for i in 4..data.len() - 3 {
        if data[i] == 0x53 && data[i + 1] == 0x41 {
            return true;
        }
    }
    false
}

pub(crate) fn is_open_frame(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // Look for Open (0x10) performative
    for i in 4..data.len() - 3 {
        if data[i] == 0x53 && data[i + 1] == 0x10 {
            return true;
        }
    }
    false
}

pub(crate) fn is_begin_frame(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // Look for Begin (0x11) performative
    for i in 4..data.len() - 3 {
        if data[i] == 0x53 && data[i + 1] == 0x11 {
            return true;
        }
    }
    false
}

pub(crate) fn is_attach_frame(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // Look for Attach (0x12) performative
    for i in 4..data.len() - 3 {
        if data[i] == 0x53 && data[i + 1] == 0x12 {
            return true;
        }
    }
    false
}

pub(crate) fn is_transfer_frame(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // Look for Transfer (0x14) performative
    for i in 4..data.len() - 3 {
        if data[i] == 0x53 && data[i + 1] == 0x14 {
            return true;
        }
    }
    false
}
