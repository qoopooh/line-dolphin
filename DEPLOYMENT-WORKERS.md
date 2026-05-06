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

### GitHub Flow over SSH to the VPS

If you want `git push origin main` to deploy from your VPS, this repo now includes
[`deploy.yml`](.github/workflows/deploy.yml), which uses GitHub Actions to SSH
into your VPS and run `wrangler deploy` there.

The intended flow is:

1. GitHub detects a push to `main`
2. GitHub starts the workflow on `ubuntu-latest`
3. The workflow SSHes into your VPS
4. The VPS updates the repo checkout
5. The VPS runs `wrangler deploy`

This keeps the actual deploy execution on your machine while GitHub only acts as the trigger.

### 0. Prepare SSH Access from GitHub Actions

Add these GitHub repository secrets:

```text
VPS_HOST
VPS_USER
VPS_SSH_KEY
```

`VPS_SSH_KEY` should be the private key GitHub Actions will use. Put the matching public key in `~/.ssh/authorized_keys` for `VPS_USER` on the server.

The workflow currently deploys from:

```text
$HOME/git/line-dolphin
```

on the VPS, so make sure this repository already exists there and `git pull --ff-only origin main` works for that checkout.

### 1. Prepare the VPS

```bash
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
rustup target add wasm32-unknown-unknown

curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs
```

You do not need to install `worker-build` or `wrangler` manually; the workflow installs them if missing.

### 2. Install Wrangler CLI

```bash
npm install -g wrangler
```

### 3. Login to Cloudflare

```bash
npx wrangler login
```

Because the deploy runs on your VPS, this login happens once on the VPS itself. GitHub does not need your Cloudflare token in this SSH-based setup.

### 4. Create KV Namespace

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

### 5. Set Secrets

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

### 6. Install worker-build

```bash
cargo install worker-build
```

### 7. Confirm Cargo.toml is already the Workers version

This repository is already configured for Cloudflare Workers. You do not need to swap in a separate `Cargo.toml`.

```bash
grep '^name = ' Cargo.toml
```

You should see:

```toml
name = "line-dolphin-worker"
```

### 8. Build and Deploy

```bash
# Deploy to Cloudflare Workers
npx wrangler deploy
```

With the GitHub flow configured, deployment becomes:

```bash
git checkout -b my-change
# make changes
git commit -am "..."
git push origin my-change
# open and merge PR
git checkout main
git pull --ff-only
git push origin main
```

Or, if your main branch is protected and merged through PRs, the deploy runs automatically when GitHub updates `main`.

### 9. First GitHub-Triggered Deploy

After adding the SSH secrets and verifying the VPS checkout:

```bash
git push origin main
```

Then check:

1. GitHub: `Actions -> Deploy Worker`
2. The SSH step output in the workflow logs
3. The deployed worker URL in the workflow logs

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

**Skip Signature Verification in Dev Mode:**

To skip LINE signature verification during local development, you can:

1. **Option 1: Use `.dev.vars` file** (recommended for local dev):
   ```bash
   # Create .dev.vars file in project root
   echo "SKIP_SIGNATURE_VERIFICATION=true" > .dev.vars
   ```

2. **Option 2: Set via command line**:
   ```bash
   npx wrangler dev --var SKIP_SIGNATURE_VERIFICATION:true
   ```

3. **Option 3: Add to `wrangler.toml`** (uncomment the line in `[vars]` section):
   ```toml
   [vars]
   SKIP_SIGNATURE_VERIFICATION = "true"
   ```

**Note:** LINE webhooks require HTTPS, so you'll need a tool like ngrok for local testing:

```bash
# In another terminal
ngrok http 8787

# Use the ngrok HTTPS URL for LINE webhook
https://abc123.ngrok.io/webhook
```

⚠️ **Important:** Never set `SKIP_SIGNATURE_VERIFICATION=true` in production! Always verify signatures in deployed workers.

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

## Reduce KV Writes (optional)

The repeated-message echo feature writes to KV on almost every group message (`msg_history:<group_id>`). If you're hitting the free-tier 1,000-writes/day limit, set the `DISABLE_REPEAT_DETECTION` env var to `true`. The bot still answers `@dolphin` / `@all` / `@on` / `@off` normally; only the "echo previous message in lowercase when a different user repeats it" behavior is skipped, and `msg_history:*` is never read or written.

`DISABLE_REPEAT_DETECTION` is a plain (non-secret) var, so it's set via `[vars]` in `wrangler.toml` or via `--var` at deploy time — **not** via `wrangler secret put`.

### Option A — commit it in `wrangler.toml` (persists across deploys)

Uncomment the line in the `[vars]` section:

```toml
[vars]
DISABLE_REPEAT_DETECTION = "true"
```

Then redeploy:

```bash
npx wrangler deploy
```

### Option B — pass at deploy time without editing the file

```bash
npx wrangler deploy --var DISABLE_REPEAT_DETECTION:true
```

Note: `--var` overrides `[vars]` for that single deploy. The next `wrangler deploy` without the flag falls back to whatever is in `wrangler.toml`.

### Option C — Cloudflare Dashboard

Workers & Pages → your worker → **Settings → Variables and Secrets** → add `DISABLE_REPEAT_DETECTION = true` as a plaintext variable. ⚠️ The next `wrangler deploy` will overwrite dashboard-only vars with whatever is in `wrangler.toml`, so prefer Option A for anything you want to survive deploys.

### Local dev

Add it to `.dev.vars` alongside `SKIP_SIGNATURE_VERIFICATION`:

```bash
echo "DISABLE_REPEAT_DETECTION=true" >> .dev.vars
```

Or pass it inline:

```bash
npx wrangler dev --var DISABLE_REPEAT_DETECTION:true
```

### Verify it's active

After deploying, send a non-command message in a group and confirm via `npx wrangler tail` that no `msg_history:*` writes occur, and check KV:

```bash
npx wrangler kv key list --binding DOLPHIN_REPLY_STATE | grep msg_history
```

No new keys should appear over time. To re-enable, remove the var (or set `"false"`) and redeploy.

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
