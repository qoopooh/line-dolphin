# Cloudflare Workers Deployment Guide

Guide to deploy the Dolphin Oracle bot to Cloudflare Workers using Rust.

## Prerequisites

- LINE Bot channel with Channel Access Token and Channel Secret
- Cloudflare account (free tier works)
- **Rust 1.88.0 or higher** (check with `rustc --version`)
- Node.js and npm installed (for Wrangler CLI)

### Check Rust Version

```bash
rustc --version
```

If your Rust version is below 1.88.0, update it:

```bash
rustup update stable
```

## Setup Steps

### 1. Install Wrangler CLI

```bash
npm install -g wrangler
```

### 2. Login to Cloudflare

```bash
npx wrangler login
```

### 3. Create KV Namespace

Create a KV namespace for storing reply state:

```bash
# Production namespace
npx wrangler kv namespace create DOLPHIN_REPLY_STATE

# Preview namespace (for testing)
npx wrangler kv namespace create DOLPHIN_REPLY_STATE --preview
```

This will output namespace IDs. Copy them and update `wrangler.toml`:

```toml
[[kv_namespaces]]
binding = "DOLPHIN_REPLY_STATE"
id = "your-production-kv-id-here"
preview_id = "your-preview-kv-id-here"
```

### 4. Set Secrets

Set your LINE credentials and broadcast configurations:

```bash
# Required secrets
npx wrangler secret put LINE_CHANNEL_ACCESS_TOKEN
# Enter your LINE channel access token when prompted

npx wrangler secret put LINE_CHANNEL_SECRET
# Enter your LINE channel secret when prompted

# Optional: Broadcast configurations (format: user_id:group_id)
npx wrangler secret put DOLPHIN_USER_TO_GROUP1
# Example: Uxxxxxxxxxxxxxxxxxxxxxxxxxxxxx:Cxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

npx wrangler secret put DOLPHIN_USER_TO_GROUP2
# Add more as needed (up to DOLPHIN_USER_TO_GROUP10)
```

### 5. Install worker-build

```bash
cargo install worker-build
```

### 6. Replace Cargo.toml

The Workers version uses a different Cargo.toml. Replace your current one:

```bash
cp Cargo.workers.toml Cargo.toml
```

Or manually update `Cargo.toml` to match `Cargo.workers.toml`.

### 7. Build and Deploy

```bash
# Deploy to Cloudflare Workers
npx wrangler deploy
```

After deployment, you'll get a URL like:
```
https://line-dolphin.<your-subdomain>.workers.dev
```

## Webhook Setup

1. Go to [LINE Developers Console](https://developers.line.biz/)
2. Select your bot channel
3. Set webhook URL: `https://line-dolphin.<your-subdomain>.workers.dev/webhook`
4. Enable "Use webhook"
5. Click "Verify" to test the connection

## Development

### Unit Tests

```bash
cargo test --lib
```

### Local Testing

```bash
# Run local development server
npx wrangler dev
```

This starts a local server at `http://localhost:8787`.

**Note:** LINE webhooks require HTTPS, so you'll need a tool like ngrok for local testing:

```bash
# In another terminal
ngrok http 8787

# Use the ngrok HTTPS URL for LINE webhook
https://abc123.ngrok.io/webhook
```

### View Logs

```bash
# Stream live logs
npx wrangler tail
```

### Update Secrets

```bash
# Update an existing secret
npx wrangler secret put LINE_CHANNEL_ACCESS_TOKEN
```

### List Secrets

```bash
# List all secrets (values are hidden)
npx wrangler secret list
```

## Key Differences from Axum Version

| Feature | Axum Version | Workers Version |
|---------|-------------|-----------------|
| Runtime | Tokio async | Workers runtime |
| HTTP Server | Axum router | Workers Router |
| HTTP Client | reqwest | Fetch API |
| State Storage | File system | KV Store |
| Environment | env::var() | env.secret() |
| Logging | tracing | console_log!() |
| Build Output | Binary | WebAssembly |

## Features

All original features are supported:

- ✅ Webhook signature verification
- ✅ Direct message replies (yes/no based on checksum)
- ✅ Group message replies (with @dolphin prefix)
- ✅ Broadcast messages (@all command)
- ✅ Targeted broadcast (@all+XXXX command)
- ✅ Reply on/off toggle (@on/@off commands)
- ✅ Special "buy nuclear" logic

## Monitoring

### View Deployment Info

```bash
npx wrangler deployments list
```

### Check Worker Status

```bash
npx wrangler whoami
```

### Analytics

View analytics in [Cloudflare Dashboard](https://dash.cloudflare.com/):
- Navigate to Workers & Pages
- Select your worker
- View the "Metrics" tab

## Troubleshooting

### Build Fails

If `worker-build` installation fails:

```bash
# Check Rust version (must be 1.88.0 or higher)
rustc --version

# Update Rust if needed
rustup update stable

# Install worker-build with locked dependencies
cargo install worker-build --locked

# If still failing, try with verbose output
cargo install worker-build --locked --verbose
```

**Common Error:** "rustc 1.86.0 is not supported"
- **Solution:** Update Rust to 1.88.0+: `rustup update stable`

### Deployment Fails

```bash
# Check wrangler version (should be 3.x or higher)
npx wrangler --version

# Update wrangler
npm install -g wrangler@latest
```

### KV Store Issues

```bash
# List KV namespaces
npx wrangler kv:namespace list

# Test KV access
npx wrangler kv:key put --binding DOLPHIN_REPLY_STATE "enabled" "enabled" --preview false
npx wrangler kv:key get --binding DOLPHIN_REPLY_STATE "enabled" --preview false
```

### Webhook Not Working

1. Check webhook URL is correct (must be HTTPS)
2. Verify secrets are set: `npx wrangler secret list`
3. Check logs: `npx wrangler tail`
4. Test signature verification with LINE's webhook test

## Cost Estimate

Cloudflare Workers Free Tier:
- **100,000 requests/day** (free)
- **10ms CPU time per request** (free)
- **KV: 100,000 reads/day** (free)
- **KV: 1,000 writes/day** (free)

For most LINE bots, this is more than enough and completely free.

## Rollback

If you need to rollback to a previous version:

```bash
# List deployments
npx wrangler deployments list

# Rollback to a specific version
npx wrangler rollback [deployment-id]
```

## Security

- ✅ Webhook signature verification enabled
- ✅ Secrets stored securely (not in code)
- ✅ HTTPS enforced by Cloudflare
- ✅ No file system access (more secure than traditional servers)

## Support

- [Cloudflare Workers Docs](https://developers.cloudflare.com/workers/)
- [workers-rs GitHub](https://github.com/cloudflare/workers-rs)
- [LINE Messaging API Docs](https://developers.line.biz/en/docs/messaging-api/)
