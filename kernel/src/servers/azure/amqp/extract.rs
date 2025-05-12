type Address = String;
#[derive(Debug)]
pub(crate) enum EntityType {
    Queue(Address),
    Topic(Address),
    Subscription(Address),
}

pub(crate) fn extract_client_info(data: &[u8]) -> Option<EntityType> {
    // This is a simplified extraction - in real code you would decode the AMQP frames

    let text = String::from_utf8_lossy(data);

    if text.contains("queue:") {
        if let Some(idx) = text.find("queue:") {
            return Some(EntityType::Queue(extract_address(&text[idx..])));
        }
    } else if text.contains("topic:") {
        if let Some(idx) = text.find("topic:") {
            let address = extract_address(&text[idx..]);
            if address.contains("/subscriptions/") {
                return Some(EntityType::Subscription(address));
            } else {
                return Some(EntityType::Topic(address));
            }
        }
    }

    None
}

pub(crate) fn extract_address(text: &str) -> String {
    // Extract address part from text like "queue:myqueue" or "topic:mytopic"
    if let Some(end) = text.find(|c: char| c.is_whitespace() || c == ';' || c == '"') {
        text[..end].to_string()
    } else {
        text.to_string()
    }
}

pub(crate) fn extract_message_data(frame_data: &[u8]) -> Option<Vec<u8>> {
    // In a real implementation, this would properly decode AMQP frames
    // Simplified extraction that skips headers
    if frame_data.len() < 30 {
        return None;
    }

    // Skip the transfer frame headers and delivery tag
    Some(frame_data[20..].to_vec())
}

pub(crate) fn extract_delivery_id(frame_data: &[u8]) -> Option<u32> {
    // Try to find the delivery ID in a transfer frame
    if frame_data.len() < 16 {
        return None;
    }

    // Look for Transfer (0x14) performative and then read delivery ID
    for i in 4..frame_data.len() - 7 {
        if frame_data[i] == 0x53 && frame_data[i + 1] == 0x14 {
            // Delivery ID should be 4 bytes after the performative
            let id_pos = i + 3;
            if id_pos + 4 <= frame_data.len() {
                let delivery_id = u32::from_be_bytes([
                    frame_data[id_pos],
                    frame_data[id_pos + 1],
                    frame_data[id_pos + 2],
                    frame_data[id_pos + 3],
                ]);
                return Some(delivery_id);
            }
        }
    }

    None
}
