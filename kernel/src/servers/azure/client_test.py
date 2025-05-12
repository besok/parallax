from azure.servicebus import ServiceBusClient, ServiceBusMessage
import logging
import time

# Enable detailed logging
logging.basicConfig(level=logging.DEBUG)

# Connection string
conn_str = "Endpoint=sb://127.0.0.1:5672/;SharedAccessKeyName=mock;SharedAccessKey=mock_key;SharedAccessKeyName=RootManageSharedAccessKey"

topic_name = "test-topic"
subscription_name = "sub1"  # Your subscription name

try:
    # Create a client
    with ServiceBusClient.from_connection_string(conn_str) as client:
        # Send a message to the topic
        topic_sender = client.get_topic_sender(topic_name)
        print("Connected successfully!")

        receiver = client.get_subscription_receiver(
            topic_name=topic_name,
            subscription_name=subscription_name,
            max_wait_time=5  # Wait up to 5 seconds for messages
        )

        print(f"Receiving messages from {topic_name}/{subscription_name}...")

        # Receive and process messages
        received_msgs = receiver.receive_messages(max_message_count=10, max_wait_time=5)

        for msg in received_msgs:
            print(f"Received: {msg}")
            print(f"Body: {str(msg.body.decode())}")

            # Complete the message to remove it from the subscription
            receiver.complete_message(msg)

except Exception as e:
    print(f"Error: {e}")
    import traceback

    traceback.print_exc()
