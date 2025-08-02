# LINE Echo Bot

A simple LINE Messaging API echo bot implemented in Rust that echoes back any text messages sent to it.

## Features

- Receives webhook events from LINE Messaging API
- Echoes back text messages to users
- Built with Rust and Axum web framework
- Structured logging with tracing
- Environment-based configuration

## Prerequisites

- Rust (latest stable version)
- A LINE Bot account and channel access token

## Setup

### 1. Create a LINE Bot

1. Go to the [LINE Developers Console](https://developers.line.biz/)
2. Create a new provider or use an existing one
3. Create a new Messaging API channel
4. Note down your **Channel Access Token** and **Channel Secret**

### 2. Configure Environment Variables

Copy the example environment file and configure it:

```bash
cp env.example .env
```

Edit `.env` and add your LINE Channel Access Token:

```env
LINE_CHANNEL_ACCESS_TOKEN=your_actual_channel_access_token_here
PORT=3000
```

### 3. Build and Run

```bash
# Build the project
cargo build --release

# Run the bot
cargo run --release
```

The bot will start on `http://localhost:3000` (or the port specified in your `.env` file).

### 4. Configure Webhook URL

1. In your LINE Developers Console, go to your Messaging API channel
2. Set the webhook URL to: `https://your-domain.com/webhook`
3. Enable "Use webhook" option
4. Add your webhook endpoint to the allowed list

**For local development**, you can use tools like:
- [ngrok](https://ngrok.com/) to expose your local server
- [localtunnel](https://github.com/localtunnel/localtunnel) for temporary public URLs

Example with ngrok:
```bash
ngrok http 3000
# Use the provided HTTPS URL + /webhook as your webhook URL
```

## API Endpoints

- `POST /webhook` - Receives LINE webhook events

## Message Flow

1. User sends a text message to your LINE Bot
2. LINE sends a webhook event to your `/webhook` endpoint
3. The bot processes the message and echoes it back
4. User receives the echoed message

## Security Considerations

For production use, you should implement:

1. **Webhook signature verification** - Verify that requests come from LINE
2. **HTTPS** - Always use HTTPS in production
3. **Rate limiting** - Implement rate limiting to prevent abuse
4. **Input validation** - Validate and sanitize incoming messages

## Development

```bash
# Run in development mode with hot reload
cargo run

# Run tests
cargo test

# Check code formatting
cargo fmt

# Run clippy for linting
cargo clippy
```

## Testing

### Local Testing

You can test the bot locally using the provided test script:

```bash
# Make sure the bot is running first
cargo run

# In another terminal, run the test script
python3 test_webhook.py "Hello, bot!"

# Or test with a custom message
python3 test_webhook.py "This is a test message"
```

The test script simulates LINE webhook events and sends them to your local bot instance.

### Manual Testing with curl

You can also test the webhook endpoint manually using curl:

```bash
curl -X POST http://localhost:3000/webhook \
  -H "Content-Type: application/json" \
  -H "X-Line-Signature: test-signature" \
  -d '{
    "events": [
      {
        "type": "message",
        "message": {
          "type": "text",
          "id": "test-id",
          "text": "Hello from curl!"
        },
        "reply_token": "test-reply-token",
        "source": {
          "type": "user",
          "userId": "test-user"
        },
        "timestamp": 1234567890
      }
    ]
  }'
```

## Dependencies

- **axum** - Web framework
- **tokio** - Async runtime
- **serde** - Serialization/deserialization
- **reqwest** - HTTP client
- **tracing** - Structured logging
- **dotenv** - Environment variable management

## License

This project is open source and available under the [MIT License](LICENSE).

## References

- [LINE Messaging API Documentation](https://developers.line.biz/en/docs/messaging-api/)
- [LINE OpenAPI Specifications](https://github.com/line/line-openapi) 