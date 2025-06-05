import argparse
import signal
import requests
from azure.servicebus import ServiceBusClient, ServiceBusMessage

# Set up argument parsing
parser = argparse.ArgumentParser(description='Receive messages from Azure Service Bus topic')
parser.add_argument('--topic', type=str, required=True, help='Topic name')
parser.add_argument('--sub', type=str, required=True, help='Subscription')
parser.add_argument('--conn', type=str, help='Connection string to Azure Service Bus',
                    default="Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;")
parser.add_argument('--ret', type=str, required=True, help='HTTP endpoint to forward received messages to')
args = parser.parse_args()

# Set up graceful shutdown
running = True

print(f"Starting continuous receiver for {args.topic}/{args.sub}")
print(f"Forwarding messages to: {args.ret}")


def forward_to_api(message_bytes):
    try:
        url = args.ret

        headers = {
            'Content-Type': 'application/octet-stream'
        }

        # Send raw bytes in the request body
        response = requests.post(url, data=message_bytes, headers=headers)

        if 200 <= response.status_code < 300:
            print(f"  Successfully forwarded to {url}, status: {response.status_code}")
            return True
        else:
            print(f"  Failed to forward to {url}, status: {response.status_code}, response: {response.text}")
            return False
    except Exception as e:
        print(f"  Error forwarding message to {url}: {e}")
        return False


# Main receiver loop
try:
    # Create a client outside the loop to maintain connection
    with ServiceBusClient.from_connection_string(
            args.conn,
            idle_timeout=280
    ) as client:
        # The receiver will wait indefinitely when max_wait_time is None
        receiver = client.get_subscription_receiver(
            topic_name=args.topic,
            subscription_name=args.sub,
            max_wait_time=None
        )

        with receiver:
            while running:
                try:
                    messages = receiver.receive_messages(max_message_count=10, max_wait_time=None)

                    for msg in messages:
                        try:
                            # Get raw bytes without decoding
                            body_bytes = b''.join(chunk for chunk in msg.body)
                            print(f"Received message: {len(body_bytes)} bytes")

                            forward_success = forward_to_api(body_bytes)

                            if forward_success:
                                receiver.complete_message(msg)
                            else:
                                receiver.abandon_message(msg)
 

                        except Exception as process_error:
                            print(f"Error processing message: {process_error}")
                            receiver.abandon_message(msg)

                except Exception as loop_error:
                    print(f"Unexpected error: {loop_error}")
                    if not running:
                        break

except Exception as e:
    print(f"Fatal error: {e}")
finally:
    print("Receiver stopped")
