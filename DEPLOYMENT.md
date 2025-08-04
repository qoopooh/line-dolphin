# Deployment Guide

Quick guide to deploy the Dolphin Oracle bot.

## Prerequisites

- LINE Bot channel with Channel Access Token and Channel Secret
- Public HTTPS endpoint for webhook

## Environment Variables

```bash
LINE_CHANNEL_ACCESS_TOKEN=your_channel_access_token
LINE_CHANNEL_SECRET=your_channel_secret
PORT=3000  # Optional
```

## Quick Deploy

### Docker (Recommended)

Create a `Dockerfile` in your project root, then run:

```bash
docker build -t dolphin-bot .
docker run -p $PORT:3000 \
  -e LINE_CHANNEL_ACCESS_TOKEN=$LINE_CHANNEL_ACCESS_TOKEN \
  -e LINE_CHANNEL_SECRET=$LINE_CHANNEL_SECRET \
  dolphin-bot
```

## Webhook Setup

1. Go to LINE Developers Console
2. Set webhook URL: `https://your-domain.com/webhook`
3. Enable "Use webhook"

## Development

### Code Formatting

Format the source code using rustfmt:

```bash
rustfmt --edition 2021 */*.rs
```

## Security

- Use HTTPS in production
- Verify webhook signatures
- Set up rate limiting
