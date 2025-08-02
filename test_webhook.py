#!/usr/bin/env python3
"""
Simple test script to simulate LINE webhook events for testing the echo bot.
Run this script to send test messages to your local bot.
"""

import json
import requests
import sys

def send_test_webhook(message_text, webhook_url="http://localhost:3001/webhook"):
    """Send a test webhook event to the bot."""
    
    # Simulate a LINE webhook event
    webhook_event = {
        "events": [
            {
                "type": "message",
                "message": {
                    "type": "text",
                    "id": "test-message-id",
                    "text": message_text
                },
                "reply_token": "test-reply-token",
                "source": {
                    "type": "user",
                    "userId": "test-user-id"
                },
                "timestamp": 1234567890
            }
        ]
    }
    
    # Headers that LINE would send
    headers = {
        "Content-Type": "application/json",
        "X-Line-Signature": "test-signature"  # In real implementation, this would be a valid signature
    }
    
    try:
        response = requests.post(webhook_url, json=webhook_event, headers=headers)
        print(f"Status Code: {response.status_code}")
        print(f"Response: {response.text}")
        
        if response.status_code == 200:
            print("✅ Webhook sent successfully!")
        else:
            print("❌ Webhook failed!")
            
    except requests.exceptions.ConnectionError:
        print("❌ Could not connect to the bot. Make sure it's running on localhost:3000")
    except Exception as e:
        print(f"❌ Error: {e}")

def main():
    if len(sys.argv) > 1:
        message = " ".join(sys.argv[1:])
    else:
        message = "Hello, LINE Echo Bot!"
    
    print(f"Sending test message: '{message}'")
    send_test_webhook(message)

if __name__ == "__main__":
    main() 
