# Deployment Guide

This guide covers deploying the LINE Echo Bot to various platforms.

## Prerequisites

Before deploying, ensure you have:

1. A LINE Bot channel with Channel Access Token and Channel Secret
2. A publicly accessible HTTPS endpoint for your webhook
3. Environment variables configured
4. For cross-compilation from macOS: `cross` tool installed (`cargo install cross`)

## Environment Variables

Set these environment variables in your deployment platform:

```bash
LINE_CHANNEL_ACCESS_TOKEN=your_channel_access_token
LINE_CHANNEL_SECRET=your_channel_secret
PORT=3000  # Optional, defaults to 3000
```

## Cross-Compilation from macOS

If you're developing on macOS (especially Apple Silicon) and need to build amd64 binaries for deployment:

### Using Cross Tool

1. Install cross: `cargo install cross`
2. Build for Linux amd64:
   ```bash
   cross build --release --target x86_64-unknown-linux-gnu
   ```
3. The binary will be available at `target/x86_64-unknown-linux-gnu/release/line-dolphin`

### Alternative: Using Docker

If you prefer using Docker for cross-compilation:

```bash
docker run --rm -v "$(pwd)":/app -w /app rust:1.88 sh -c "rustup target add x86_64-unknown-linux-gnu && cargo build --release --target x86_64-unknown-linux-gnu"
```

## Deployment Options

### 1. Docker Deployment

Create a `Dockerfile`:

```dockerfile
FROM --platform=linux/amd64 rust:1.88 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-gnu

FROM --platform=linux/amd64 debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/line-dolphin /usr/local/bin/line-dolphin

EXPOSE 3000
CMD ["line-dolphin"]
```

Build and run:

```bash
# Option 1: Build using Docker (recommended for consistency)
docker build -t line-echo-bot .
docker run -p 3000:3000 \
  -e LINE_CHANNEL_ACCESS_TOKEN=your_token \
  -e LINE_CHANNEL_SECRET=your_secret \
  line-echo-bot
```

## Webhook Configuration

After deployment, configure your LINE Bot webhook:

1. Go to LINE Developers Console
2. Set webhook URL to: `https://your-domain.com/webhook`
3. Enable "Use webhook"
4. Add your domain to allowed webhook endpoints

## Monitoring and Logging

### Health Check

The bot provides a health check endpoint at `/` that returns HTTP 200 when running.

### Logging

The bot uses structured logging with tracing. In production, consider:

- Sending logs to a centralized logging service
- Setting appropriate log levels
- Monitoring error rates and response times

### Metrics

Consider adding metrics collection for:
- Webhook request count
- Response times
- Error rates
- Message processing success/failure

## Security Considerations

1. **HTTPS Only**: Always use HTTPS in production
2. **Webhook Verification**: Implement proper signature verification
3. **Rate Limiting**: Add rate limiting to prevent abuse
4. **Input Validation**: Validate all incoming webhook data
5. **Secrets Management**: Use secure secret management (not environment variables in production)

## Troubleshooting

### Common Issues

1. **Webhook not receiving events**
   - Check webhook URL is correct and accessible
   - Verify HTTPS is enabled
   - Check firewall/security group settings

2. **Bot not responding**
   - Verify Channel Access Token is correct
   - Check bot is added to conversations
   - Review logs for API errors

3. **Signature verification failures**
   - Ensure Channel Secret is correct
   - Verify signature calculation logic
   - Check request body format

### Debug Mode

For debugging, run with verbose logging:

```bash
RUST_LOG=debug cargo run
```

## Performance Optimization

1. **Connection Pooling**: Reuse HTTP connections
2. **Async Processing**: Handle multiple webhooks concurrently
3. **Caching**: Cache frequently accessed data
4. **Resource Limits**: Set appropriate memory and CPU limits

## Backup and Recovery

1. **Configuration Backup**: Store configuration securely
2. **Database Backup**: If using persistent storage
3. **Deployment Rollback**: Plan for quick rollback procedures
4. **Monitoring Alerts**: Set up alerts for service failures 