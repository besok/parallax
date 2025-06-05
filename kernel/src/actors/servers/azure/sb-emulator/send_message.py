from azure.servicebus import ServiceBusClient, ServiceBusMessage, TransportType
import argparse
import sys
import base64

# Parse command line arguments
parser = argparse.ArgumentParser(description='Send a message to Azure Service Bus topic')
parser.add_argument('--topic', type=str, required=True, help='Topic name')
parser.add_argument('--message', type=str, help='Message content as string')
parser.add_argument('--base64', type=str, help='Message content as base64 encoded string')
parser.add_argument('--conn', type=str, help='Connction string to Azure Service Bus',
                    default="Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;")
args = parser.parse_args()

if not args.message and not args.base64:
    print("Error: Either --message or --base64 must be provided")
    sys.exit(1)

try:
    # Create a message either from plain text or base64-encoded bytes
    if args.message:
        message_content = args.message
    else:
        # Decode the base64 string to bytes
        message_content = base64.b64decode(args.base64)

    # Create a client
    with ServiceBusClient.from_connection_string(args.conn) as client:
        # Send the message to the specified topic
        print(f"Connecting to send message to topic '{args.topic}'...")
        topic_sender = client.get_topic_sender(args.topic)

        topic_message = ServiceBusMessage(message_content)
        topic_sender.send_messages(topic_message)
        print("Message sent successfully!")

except Exception as e:
    print(f"Error: {e}")
    import traceback

    traceback.print_exc()
    sys.exit(1)

sys.exit(0)
