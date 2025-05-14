from azure.servicebus import ServiceBusClient, ServiceBusMessage, TransportType

import logging
import ssl
import time

# Enable detailed logging
# logging.basicConfig(level=logging.DEBUG)

conn_str = "Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;"

topic_name = "test-topic"
subscription_name = "test-sub"  # Your subscription name

try:
    # Create a client with TransportType.Amqp (TCP mode)
    with ServiceBusClient.from_connection_string(
            conn_str
    ) as client:
        # # First send a message to the topic
        # print("Attempting to connect and send a message...")
        # topic_sender = client.get_topic_sender(topic_name)
        #
        # topic_message = ServiceBusMessage("Test message content")
        # topic_sender.send_messages(topic_message)
        # print("Message sent successfully!")

        # Then try to receive messages
        print(f"Creating receiver for {topic_name}/{subscription_name}...")
        receiver = client.get_subscription_receiver(
            topic_name=topic_name,
            subscription_name=subscription_name,
            max_wait_time=5
        )

        print(f"Waiting for messages...")
        received_msgs = receiver.receive_messages(max_message_count=10, max_wait_time=5)

        if not received_msgs:
            print("No messages received within the timeout period")

        for msg in received_msgs:
            print(f"Received: {msg}")
            body_content = b''.join(chunk for chunk in msg.body)
            body_str = body_content.decode('utf-8') if isinstance(body_content, bytes) else str(body_content)
            print(f"Body: {str(body_str)}")
            receiver.complete_message(msg)

except Exception as e:
    print(f"Error: {e}")
    import traceback

    traceback.print_exc()
