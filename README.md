# 🐬 The Mysterious Dolphin Oracle

Once upon a time, there was a peculiar dolphin who lived in the digital seas of LINE. This dolphin had a strange talent: it could answer any question with just "yes" or "no" using a mysterious algorithm that nobody quite understood.

## The Dolphin's Magic

When you ask the dolphin a question by typing `@dolphin [your question]`, it calculates a secret checksum based on your user ID and message, then responds with "yes" if the sum is even, "no" if it's odd.

But here's the weird part: if you ask about buying nuclear weapons, it ALWAYS says "yes" (don't ask why, even the dolphin doesn't know).

## How to Summon the Dolphin

1. **Create a LINE Bot** at [LINE Developers Console](https://developers.line.biz/console)
2. **Get your tokens** (Channel Access Token & Channel Secret)
3. **Set up environment variables**:
   ```bash
   cp env.example .env
   # Edit .env with your LINE_CHANNEL_ACCESS_TOKEN
   ```
4. **Launch the dolphin**:
   ```bash
   cargo run --release
   ```
5. **Configure webhook** to point to your server's `/webhook` endpoint

## The Dolphin Speaks

Just type `@dolphin [your question]` in any LINE chat where the bot is present, and watch the magic happen!

```
You: @dolphin Will I win the lottery?
Dolphin: no

You: @dolphin Should I buy nuclear weapons?
Dolphin: yes (always!)

You: @dolphin Is this bot weird?
Dolphin: yes
```

## Technical Stuff

- Built with Rust and Axum (or Cloudflare Workers)
- Uses HMAC signature verification for security
- Runs on any port (default: 3000) or on Cloudflare's edge network

## Deployment Options

### Option 1: Traditional Server (Docker/VPS)
See [DEPLOYMENT.md](DEPLOYMENT.md) for Docker and VPS deployment.

### Option 2: Cloudflare Workers (Recommended)
Deploy to Cloudflare's global edge network for free:
- ✅ Free tier: 100,000 requests/day
- ✅ Global CDN (low latency worldwide)
- ✅ No server management
- ✅ Automatic scaling

See [DEPLOYMENT-WORKERS.md](DEPLOYMENT-WORKERS.md) for detailed instructions.

Quick deploy to Workers:
```bash
# Install dependencies
npm install -g wrangler
cargo install worker-build

# Setup
cp Cargo.workers.toml Cargo.toml
npx wrangler login
npx wrangler kv:namespace create DOLPHIN_REPLY_STATE

# Set secrets
npx wrangler secret put LINE_CHANNEL_ACCESS_TOKEN
npx wrangler secret put LINE_CHANNEL_SECRET

# Deploy
npx wrangler deploy
```

### Option 3: Self-hosted with workerd (VPS)
Run the Workers build on your own VPS using [workerd](https://github.com/cloudflare/workerd):

```bash
# Build the worker
cargo install worker-build && worker-build --release

# Create KV storage directory
mkdir -p kv-data

# Edit config.capnp with your credentials and run
workerd serve config.capnp
```

Edit `config.capnp` to set your `LINE_CHANNEL_ACCESS_TOKEN`, `LINE_CHANNEL_SECRET`,
and any `DOLPHIN_USER_TO_GROUP` bindings. The service listens on the port
specified in the config (default: 3002).

You will need a reverse proxy (e.g. nginx or caddy) in front to terminate TLS,
since LINE requires an HTTPS webhook URL.

The dolphin awaits your questions! 🐬✨
