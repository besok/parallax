mod extract;
mod frames;

use crate::servers::ServerError;
use crate::servers::azure::amqp::extract::*;
use crate::servers::azure::amqp::frames::*;
use crate::servers::azure::{BusState, ServiceBusMessage};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;

pub async fn handle_amqp_connection(
    stream: tokio::net::TcpStream,
    state: BusState,
) -> Result<(), ServerError> {
    log::info!("New AMQP connection established");

    if let Err(e) = stream.set_nodelay(true) {
        log::warn!("Failed to set TCP_NODELAY: {}", e);
    }

    let (mut read_half, mut write_half) = stream.into_split();
    let mut buffer = vec![0u8; 8192];

    let mut connection_established = false;
    let mut sasl_negotiated = false;
    let mut session_begun = false;
    let mut link_established = false;
    let mut client_info = None;

    let protocol_header = b"AMQP\x00\x01\x00\x00";
    if let Err(e) = write_half.write_all(protocol_header).await {
        return Err(ServerError::RuntimeError(format!(
            "Failed to send protocol header: {}",
            e
        )));
    }

    loop {
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            read_half.read(&mut buffer),
        )
        .await
        {
            Ok(Ok(0)) => {
                log::info!("AMQP connection closed by client");
                break;
            }
            Ok(Ok(n)) => {
                log::debug!("Read {} bytes from AMQP connection", n);
                let frame_data = &buffer[..n];

                // Check if this is the initial AMQP protocol header
                if !connection_established && n >= 8 && &frame_data[0..4] == b"AMQP" {
                    connection_established = true;

                    // Send SASL mechanisms frame offering PLAIN auth
                    let sasl_mechanisms = create_sasl_mechanisms_frame();
                    if let Err(e) = write_half.write_all(&sasl_mechanisms).await {
                        log::error!("Failed to send SASL mechanisms: {}", e);
                    }
                    continue;
                }

                // Handle SASL authentication
                if connection_established && !sasl_negotiated && is_sasl_init_frame(frame_data) {
                    sasl_negotiated = true;

                    let sasl_outcome = create_sasl_outcome_frame(true);
                    if let Err(e) = write_half.write_all(&sasl_outcome).await {
                        log::error!("Failed to send SASL outcome: {}", e);
                    }
                    continue;
                }

                // Handle Open frame
                if sasl_negotiated && !session_begun && is_open_frame(frame_data) {
                    let open_response = create_open_response();
                    if let Err(e) = write_half.write_all(&open_response).await {
                        log::error!("Failed to send Open response: {}", e);
                    }
                    continue;
                }

                // Handle Begin frame
                if !session_begun && is_begin_frame(frame_data) {
                    session_begun = true;

                    let begin_response = create_begin_response();
                    if let Err(e) = write_half.write_all(&begin_response).await {
                        log::error!("Failed to send Begin response: {}", e);
                    }
                    continue;
                }

                // Handle Attach frame
                if session_begun && !link_established && is_attach_frame(frame_data) {
                    link_established = true;

                    client_info = extract_client_info(frame_data);

                    if let Some(entity) = &client_info {
                        log::info!("Client attached to {:?}'", entity);
                    }

                    let attach_response = create_attach_response(frame_data);
                    if let Err(e) = write_half.write_all(&attach_response).await {
                        log::error!("Failed to send Attach response: {}", e);
                    }

                    let flow_frame = create_flow_frame(100); // Allow 100 messages
                    if let Err(e) = write_half.write_all(&flow_frame).await {
                        log::error!("Failed to send Flow frame: {}", e);
                    }
                    continue;
                }

                // Handle Transfer frames (messages from client to server)
                if link_established && is_transfer_frame(frame_data) {
                    if let Some(entity) = &client_info {
                        let message_data = extract_message_data(frame_data);

                        if let Some(data) = message_data {
                            match entity {
                                EntityType::Queue(name) => {
                                    process_queue_message(name, &data, &state)?;

                                    let disposition = create_disposition_frame(frame_data);
                                    if let Err(e) = write_half.write_all(&disposition).await {
                                        log::error!("Failed to send disposition: {}", e);
                                    }
                                }
                                EntityType::Topic(name) => {
                                    process_topic_message(name, &data, &state)?;

                                    let disposition = create_disposition_frame(frame_data);
                                    if let Err(e) = write_half.write_all(&disposition).await {
                                        log::error!("Failed to send disposition: {}", e);
                                    }

                                    send_messages_to_subscriptions(name, &mut write_half, &state)
                                        .await?;
                                }
                                _ => {}
                            }
                        }
                    }
                    continue;
                }

                if link_established && client_info.is_some() {
                    if let Some(EntityType::Subscription(name)) = client_info.as_ref() {
                        if let Some((topic_name, subscription_name)) = parse_subscription_name(name)
                        {
                            if let Ok(sent) = send_subscription_messages(
                                topic_name,
                                subscription_name,
                                &mut write_half,
                                &state,
                            )
                            .await
                            {
                                if sent > 0 {
                                    log::info!(
                                        "Sent {} messages to subscription {}",
                                        sent,
                                        subscription_name
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                return Err(ServerError::RuntimeError(format!(
                    "Failed to read from stream: {}",
                    e
                )));
            }
            Err(_) => {
                // Timeout - check if we need to send any pending messages to subscriptions
                if let Some(EntityType::Subscription(name)) = &client_info {
                    if let Some((topic_name, subscription_name)) = parse_subscription_name(name) {
                        if let Ok(sent) = send_subscription_messages(
                            topic_name,
                            subscription_name,
                            &mut write_half,
                            &state,
                        )
                        .await
                        {
                            if sent > 0 {
                                log::info!(
                                    "Sent {} messages to subscription {}",
                                    sent,
                                    subscription_name
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_subscription_name(name: &str) -> Option<(&str, &str)> {
    // Parse a name like "topic:mytopic/subscriptions/mysub"
    if let Some(topic_part) = name.strip_prefix("topic:") {
        if let Some(idx) = topic_part.find("/subscriptions/") {
            let topic_name = &topic_part[..idx];
            let sub_name = &topic_part[idx + 14..]; // Skip "/subscriptions/"
            return Some((topic_name, sub_name));
        }
    }
    None
}

// Message handling functions

async fn send_subscription_messages(
    topic_name: &str,
    subscription_name: &str,
    writer: &mut OwnedWriteHalf,
    state: &BusState,
) -> Result<usize, ServerError> {
    let mut topics = state.topics.lock()?;

    let mut sent_count = 0;

    if let Some(subscriptions) = topics.get_mut(topic_name) {
        if let Some(messages) = subscriptions.get_mut(subscription_name) {
            if !messages.is_empty() {
                let msgs_to_send = std::mem::take(messages);
                drop(topics); // Release lock before async operations

                for (i, msg) in msgs_to_send.iter().enumerate() {
                    // Format message as AMQP transfer frame
                    let transfer = create_transfer_frame((i + 1) as u32, &msg.body);

                    if let Err(e) = writer.write_all(&transfer).await {
                        log::error!("Failed to send message to subscription: {}", e);
                        continue;
                    }

                    sent_count += 1;

                    // Add a small delay between messages
                    if i < msgs_to_send.len() - 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                }
            }
        }
    }

    Ok(sent_count)
}

async fn send_messages_to_subscriptions(
    topic_name: &str,
    writer: &mut OwnedWriteHalf,
    state: &BusState,
) -> Result<(), ServerError> {
    // Get list of active subscriptions for this topic
    let subscription_names = {
        let topics = state.topics.lock()?;

        if let Some(subscriptions) = topics.get(topic_name) {
            subscriptions.keys().cloned().collect::<Vec<String>>()
        } else {
            log::warn!(
                "Attempted to send messages for unknown topic: {}",
                topic_name
            );
            return Ok(());
        }
    };

    // Process each subscription
    for subscription_name in subscription_names {
        // Try to send any pending messages for this subscription
        match send_subscription_messages(topic_name, &subscription_name, writer, state).await {
            Ok(count) => {
                if count > 0 {
                    log::info!(
                        "Sent {} messages to subscription {} of topic {}",
                        count,
                        subscription_name,
                        topic_name
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to send messages to subscription {} of topic {}: {:?}",
                    subscription_name,
                    topic_name,
                    e
                );
            }
        }
    }

    Ok(())
}

fn process_queue_message(
    queue_name: &str,
    data: &[u8],
    state: &BusState,
) -> Result<(), ServerError> {
    let mut queues = state.queues.lock()?;

    if let Some(queue) = queues.get_mut(queue_name) {
        let message = ServiceBusMessage {
            message_id: Some(format!("msg-{}", uuid::Uuid::new_v4())),
            body: data.to_vec(),
            properties: std::collections::HashMap::new(),
            content_type: Some("application/octet-stream".to_string()),
            subject: None,
        };

        queue.push(message);
        log::info!("Message added to queue: {}", queue_name);
    } else {
        log::warn!("Message received for unknown queue: {}", queue_name);
    }

    Ok(())
}

fn process_topic_message(
    topic_name: &str,
    data: &[u8],
    state: &BusState,
) -> Result<(), ServerError> {
    let mut topics = state.topics.lock()?;

    if let Some(subscriptions) = topics.get_mut(topic_name) {
        let message = ServiceBusMessage {
            message_id: Some(format!("msg-{}", uuid::Uuid::new_v4())),
            body: data.to_vec(),
            properties: std::collections::HashMap::new(),
            content_type: Some("application/octet-stream".to_string()),
            subject: None,
        };

        // Deliver to all subscriptions
        for (subscription_name, messages) in subscriptions.iter_mut() {
            messages.push(message.clone());
            log::info!(
                "Message delivered to subscription {} of topic {}",
                subscription_name,
                topic_name
            );
        }
    } else {
        log::warn!("Message received for unknown topic: {}", topic_name);
    }

    Ok(())
}
