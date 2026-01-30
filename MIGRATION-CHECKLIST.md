# Cloudflare Workers Migration Checklist

Quick reference for migrating from Axum to Cloudflare Workers.

## ✅ Pre-Migration Checklist

- [ ] Cloudflare account created
- [ ] Node.js and npm installed
- [ ] **Rust 1.88.0+ installed** (check: `rustc --version`)
- [ ] LINE Bot credentials ready (Channel Access Token & Secret)
- [ ] Current deployment backed up (if any)

### Update Rust if Needed

```bash
# Check version
rustc --version

# Update if below 1.88.0
rustup update stable
```

## 📦 Installation

```bash
# Install Wrangler CLI
npm install -g wrangler

# Install worker-build
cargo install worker-build

# Login to Cloudflare
npx wrangler login
```

## 🔧 Configuration Steps

### 1. Create KV Namespace

```bash
# Production
npx wrangler kv:namespace create DOLPHIN_REPLY_STATE

# Preview (for testing)
npx wrangler kv:namespace create DOLPHIN_REPLY_STATE --preview
```

Copy the IDs and update `wrangler.toml`.

### 2. Update Cargo.toml

```bash
cp Cargo.workers.toml Cargo.toml
```

### 3. Set Secrets

```bash
# Required
npx wrangler secret put LINE_CHANNEL_ACCESS_TOKEN
npx wrangler secret put LINE_CHANNEL_SECRET

# Optional (broadcast feature)
npx wrangler secret put DOLPHIN_USER_TO_GROUP1
npx wrangler secret put DOLPHIN_USER_TO_GROUP2
```

### 4. Deploy

```bash
npx wrangler deploy
```

## 🔄 Key Code Changes

| Axum Version | Workers Version |
|--------------|-----------------|
| `src/main.rs` | `src/lib.rs` |
| `#[tokio::main]` | `#[event(fetch)]` |
| `env::var()` | `env.secret()` |
| `fs::read_to_string()` | `kv.get().text().await` |
| `fs::write()` | `kv.put().execute().await` |
| `reqwest::Client` | `Fetch::Request()` |
| `tracing::info!()` | `console_log!()` |
| `Router::new()` (axum) | `Router::new()` (worker) |

## 📝 Environment Variables

### Axum (.env file)
```bash
LINE_CHANNEL_ACCESS_TOKEN=xxx
LINE_CHANNEL_SECRET=xxx
DOLPHIN_USER_TO_GROUP1=user_id:group_id
PORT=3000
```

### Workers (Secrets)
```bash
# Set via wrangler secret
npx wrangler secret put LINE_CHANNEL_ACCESS_TOKEN
npx wrangler secret put LINE_CHANNEL_SECRET
npx wrangler secret put DOLPHIN_USER_TO_GROUP1
```

## 🧪 Testing

### Local Development
```bash
# Start local Workers server
npx wrangler dev

# In another terminal, use ngrok for HTTPS
ngrok http 8787
```

### View Logs
```bash
npx wrangler tail
```

### Test Webhook
```bash
curl -X POST https://line-dolphin.YOUR-SUBDOMAIN.workers.dev/webhook \
  -H "Content-Type: application/json" \
  -d '{"destination":"test","events":[]}'
```

## 🚀 Post-Deployment

- [ ] Copy Workers URL
- [ ] Update LINE webhook URL
- [ ] Enable webhook in LINE console
- [ ] Verify webhook connection
- [ ] Test with a message
- [ ] Monitor logs (`npx wrangler tail`)

## 🐛 Troubleshooting

### Build Errors

**Error: "rustc 1.86.0 is not supported"**
```bash
# Update Rust to 1.88.0+
rustup update stable

# Verify version
rustc --version
```

**Other build issues:**
```bash
# Clear cargo cache
cargo clean

# Reinstall worker-build with locked dependencies
cargo uninstall worker-build
cargo install worker-build --locked
```

### Deployment Errors
```bash
# Check wrangler version
npx wrangler --version

# Update wrangler
npm install -g wrangler@latest

# Check authentication
npx wrangler whoami
```

### KV Store Issues
```bash
# List namespaces
npx wrangler kv:namespace list

# Test KV operations
npx wrangler kv:key put --binding DOLPHIN_REPLY_STATE "test" "value"
npx wrangler kv:key get --binding DOLPHIN_REPLY_STATE "test"
```

### Webhook Not Working
1. Check logs: `npx wrangler tail`
2. Verify secrets: `npx wrangler secret list`
3. Test signature verification
4. Check LINE webhook settings
5. Ensure URL is correct: `https://your-worker.workers.dev/webhook`

## 📊 Monitoring

```bash
# Live logs
npx wrangler tail

# Deployment history
npx wrangler deployments list

# Worker status
npx wrangler whoami
```

## 💰 Cost Comparison

| Platform | Cost | Requests/month |
|----------|------|----------------|
| Cloudflare Workers Free | $0 | 100K/day (3M/month) |
| VPS (Basic) | $5-10/month | Unlimited* |
| AWS Lambda Free | $0 | 1M/month |
| Heroku Basic | $7/month | Unlimited* |

*Subject to resource limits

## 🔙 Rollback

If something goes wrong:

```bash
# Restore original Cargo.toml
cp Cargo.toml.backup Cargo.toml

# Rollback to previous deployment
npx wrangler deployments list
npx wrangler rollback [deployment-id]
```

## ✨ Benefits of Workers

- ✅ **Free tier**: 100K requests/day
- ✅ **Global CDN**: Low latency worldwide
- ✅ **Auto-scaling**: No capacity planning
- ✅ **Zero maintenance**: No servers to manage
- ✅ **Built-in security**: DDoS protection included
- ✅ **Fast cold starts**: ~1ms startup time
- ✅ **Edge computing**: Runs close to users

## 📚 Resources

- [Cloudflare Workers Docs](https://developers.cloudflare.com/workers/)
- [workers-rs GitHub](https://github.com/cloudflare/workers-rs)
- [Wrangler CLI Docs](https://developers.cloudflare.com/workers/wrangler/)
- [LINE Messaging API](https://developers.line.biz/en/docs/messaging-api/)

## 🎯 Success Criteria

Migration is complete when:

- [ ] Worker deployed successfully
- [ ] Webhook URL updated in LINE console
- [ ] Test message receives correct reply
- [ ] @dolphin command works in groups
- [ ] @all broadcast works (if configured)
- [ ] @on/@off commands work (if configured)
- [ ] Logs show no errors
- [ ] Original server can be shut down

---

**Need help?** Check [DEPLOYMENT-WORKERS.md](DEPLOYMENT-WORKERS.md) for detailed instructions.
