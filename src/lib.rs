mod types;

use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use worker::*;

use types::{ReplyMessage, ReplyRequest};

#[derive(Debug, Deserialize)]
struct WebhookRequest {
    destination: String,
    events: Vec<WebhookEvent>,
}

#[derive(Debug, Deserialize)]
struct WebhookEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "webhookEventId")]
    webhook_event_id: String,
    #[serde(rename = "deliveryContext")]
    delivery_context: DeliveryContext,
    message: Option<Message>,
    #[serde(rename = "replyToken")]
    reply_token: Option<String>,
    source: Source,
    timestamp: i64,
    mode: String,
}

#[derive(Debug, Deserialize)]
struct DeliveryContext {
    #[serde(rename = "isRedelivery")]
    is_redelivery: bool,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(rename = "type")]
    message_type: String,
    id: String,
    text: Option<String>,
    #[serde(rename = "quoteToken")]
    quote_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct Source {
    #[serde(rename = "type")]
    source_type: String,
    #[serde(rename = "userId")]
    user_id: Option<String>,
    #[serde(rename = "groupId")]
    group_id: Option<String>,
    #[serde(rename = "roomId")]
    room_id: Option<String>,
}

#[derive(Debug)]
struct BroadcastConfig {
    allowed_user_id: String,
    target_group_id: String,
}

impl BroadcastConfig {
    fn from_env(env: &Env) -> Vec<Self> {
        let mut configs = Vec::new();

        // Check for numbered configurations
        for i in 1..=10 {
            let env_key = format!("DOLPHIN_USER_TO_GROUP{}", i);
            if let Ok(var) = env.secret(&env_key) {
                let val = var.to_string();
                if let Some(config) = Self::parse_config(&val) {
                    configs.push(config);
                }
            }
        }

        // Check for the original DOLPHIN_USER_TO_GROUP
        if let Ok(var) = env.secret("DOLPHIN_USER_TO_GROUP") {
            let val = var.to_string();
            if let Some(config) = Self::parse_config(&val) {
                configs.push(config);
            }
        }

        configs
    }

    fn parse_config(var: &str) -> Option<Self> {
        let parts: Vec<&str> = var.split(':').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some(BroadcastConfig {
                allowed_user_id: parts[0].to_string(),
                target_group_id: parts[1].to_string(),
            })
        } else {
            None
        }
    }

    fn is_user_authorized(&self, user_id: &str) -> bool {
        self.allowed_user_id == user_id
    }

    fn find_by_user_id<'a>(configs: &'a [Self], user_id: &str) -> Option<&'a Self> {
        configs
            .iter()
            .find(|config| config.is_user_authorized(user_id))
    }

    fn has_authorized_user(configs: &[Self]) -> bool {
        !configs.is_empty()
    }
}

fn verify_signature(body: &[u8], signature: &str, channel_secret: &str) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(channel_secret.as_bytes())
        .expect("HMAC can take key of any size");

    mac.update(body);
    let result = mac.finalize();
    let expected_signature = general_purpose::STANDARD.encode(result.into_bytes());

    signature == expected_signature
}

async fn is_replies_enabled(kv: &kv::KvStore) -> bool {
    match kv.get("enabled").text().await {
        Ok(Some(content)) => content.trim() == "enabled",
        _ => true, // Default to enabled
    }
}

async fn set_replies_enabled(kv: &kv::KvStore, enabled: bool) -> Result<()> {
    let state = if enabled { "enabled" } else { "disabled" };
    kv.put("enabled", state)?.execute().await?;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct MessageEntry {
    user_id: String,
    message: String,
}

async fn get_last_message(db: &D1Database, group_id: &str) -> Option<MessageEntry> {
    let result = db
        .prepare("SELECT user_id, message FROM msg_history WHERE group_id = ?1")
        .bind(&[group_id.into()]);
    let statement = match result {
        Ok(statement) => statement,
        Err(e) => {
            console_error!("Failed to bind msg_history query: {}", e);
            return None;
        }
    };
    match statement.first::<MessageEntry>(None).await {
        Ok(entry) => entry,
        Err(e) => {
            console_error!("Failed to read msg_history for {}: {}", group_id, e);
            None
        }
    }
}

async fn save_last_message(db: &D1Database, group_id: &str, user_id: &str, message: &str) {
    let result = db
        .prepare(
            "INSERT INTO msg_history (group_id, user_id, message, updated_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(group_id) DO UPDATE SET \
             user_id = excluded.user_id, message = excluded.message, updated_at = excluded.updated_at",
        )
        .bind(&[
            group_id.into(),
            user_id.into(),
            message.into(),
            (Date::now().as_millis() as f64).into(),
        ]);
    let outcome = match result {
        Ok(statement) => statement.run().await.map(|_| ()),
        Err(e) => Err(e),
    };
    if let Err(e) = outcome {
        console_error!("Failed to save msg_history for {}: {}", group_id, e);
    }
}

/// Returns the previous message in lowercase if a different user's message starts with it
/// (case-insensitive).
fn repeated_reply(
    last_entry: &MessageEntry,
    current_user_id: &str,
    current_message: &str,
) -> Option<String> {
    if last_entry.user_id == current_user_id {
        return None;
    }

    let previous_lower = last_entry.message.to_lowercase();
    if current_message.to_lowercase().starts_with(&previous_lower) {
        Some(previous_lower)
    } else {
        None
    }
}

async fn send_reply(
    reply_token: &str,
    text: &str,
    source: &Source,
    env: &Env,
    kv: &kv::KvStore,
    db: &D1Database,
    disable_repeat_detection: bool,
) -> Result<()> {
    let user_id = source.user_id.as_deref().unwrap_or("unknown");
    let has_group_id = source.group_id.is_some();

    let trimmed_text = text.trim().to_lowercase();
    let is_dolphin_message = trimmed_text.starts_with("@dolphin");
    let is_off_command = trimmed_text.starts_with("@off");
    let is_on_command = trimmed_text.starts_with("@on");

    // Check for @all+XXXX pattern or "@all"
    let all_plus_pattern = Regex::new(r"^@all\+(\w{4})").unwrap();
    let target_group_digits = all_plus_pattern
        .captures(&trimmed_text)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string());
    let is_all_plus_message = target_group_digits.is_some();
    let is_all_message = target_group_digits.is_none() && trimmed_text.starts_with("@all");

    // Handle @off and @on commands
    if is_off_command || is_on_command {
        let broadcast_configs = BroadcastConfig::from_env(env);
        if BroadcastConfig::has_authorized_user(&broadcast_configs) {
            if BroadcastConfig::find_by_user_id(&broadcast_configs, user_id).is_some() {
                let enable = is_on_command;
                if set_replies_enabled(kv, enable).await.is_ok() {
                    let status = if enable { "enabled" } else { "disabled" };
                    let reply_text = format!("🔧 Replies have been {}", status);
                    send_line_reply(reply_token, &reply_text, env).await?;
                    console_log!("Reply status changed to {} by user {}", status, user_id);
                    return Ok(());
                } else {
                    let reply_text = "❌ Failed to change reply status".to_string();
                    send_line_reply(reply_token, &reply_text, env).await?;
                    return Ok(());
                }
            }
        }
    }

    // Check if replies are enabled
    if !is_replies_enabled(kv).await && has_group_id {
        console_log!(
            "Replies are disabled, ignoring message from user {}",
            user_id
        );
        return Ok(());
    }

    // Check for repeated messages in group conversations
    if !disable_repeat_detection && has_group_id {
        if let Some(group_id) = &source.group_id {
            // Skip repeated message check for commands (@dolphin, @on, @off)
            if !is_dolphin_message && !is_all_plus_message && !is_off_command && !is_on_command {
                let repeated = get_last_message(db, group_id)
                    .await
                    .and_then(|last_entry| repeated_reply(&last_entry, user_id, text));
                if let Some(repeated_reply) = repeated {
                    // Update message history with current message
                    save_last_message(db, group_id, user_id, text).await;

                    // Reply with the previous message in lowercase
                    send_line_reply(reply_token, &repeated_reply, env).await?;
                    console_log!(
                        "Repeated message detected in group {}: {}",
                        group_id,
                        repeated_reply
                    );
                    return Ok(());
                }
            }
        }
    }

    if !is_dolphin_message && !is_all_plus_message {
        if has_group_id {
            // Update message history for group messages
            if !disable_repeat_detection {
                if let Some(group_id) = &source.group_id {
                    save_last_message(db, group_id, user_id, text).await;
                }
            }
            return Ok(());
        }

        let reply_text = create_reply(user_id, text);
        send_line_reply(reply_token, &reply_text, env).await?;
        console_log!("Reply to user {}: {}", user_id, text);
        return Ok(());
    }

    let broadcast_configs = BroadcastConfig::from_env(env);
    let (message_content, is_broadcast, authorized_broadcast, target_group_id) =
        if is_all_plus_message {
            let prefix_len = format!("@all+{}", target_group_digits.as_ref().unwrap()).len();
            let content = text.trim()[prefix_len..].trim();

            let target_group = broadcast_configs
                .iter()
                .find(|config| {
                    config
                        .target_group_id
                        .ends_with(target_group_digits.as_ref().unwrap().as_str())
                })
                .map(|config| config.target_group_id.clone());

            let authorized = target_group.is_some();

            (content.to_string(), true, authorized, target_group)
        } else if is_all_message {
            let content = text.trim()[4..].trim();
            let authorized =
                BroadcastConfig::find_by_user_id(&broadcast_configs, user_id).is_some();

            (content.to_string(), true, authorized, None)
        } else {
            let content = text.trim()[8..].trim();
            (content.to_string(), false, false, None)
        };

    if message_content.is_empty() {
        return Ok(());
    }

    let reply_text = create_response_msg(
        user_id,
        &message_content,
        has_group_id,
        is_broadcast,
        authorized_broadcast,
        is_all_plus_message,
        &target_group_id,
        &target_group_digits,
        &broadcast_configs,
        env,
    )
    .await;

    send_line_reply(reply_token, &reply_text, env).await?;
    let group_id = source.group_id.as_deref().unwrap_or("unknown");
    console_log!(
        "Reply sent (group_id:{}, broadcast:{}, user_id:{}): {}",
        group_id,
        is_broadcast,
        &user_id[0..4],
        message_content
    );

    // Update message history for group messages
    if !disable_repeat_detection && has_group_id {
        if let Some(group_id) = &source.group_id {
            save_last_message(db, group_id, user_id, text).await;
        }
    }

    Ok(())
}

async fn create_response_msg(
    user_id: &str,
    message_content: &str,
    has_group_id: bool,
    is_broadcast: bool,
    authorized_broadcast: bool,
    is_all_plus_message: bool,
    target_group_id: &Option<String>,
    target_group_digits: &Option<String>,
    broadcast_configs: &[BroadcastConfig],
    env: &Env,
) -> String {
    if has_group_id {
        create_reply(user_id, message_content)
    } else {
        if authorized_broadcast {
            let target_group = if let Some(group_id) = target_group_id {
                group_id.clone()
            } else {
                BroadcastConfig::find_by_user_id(broadcast_configs, user_id)
                    .map(|config| config.target_group_id.clone())
                    .unwrap_or_default()
            };

            if !target_group.is_empty() {
                if let Err(e) = send_push_message(&target_group, message_content, env).await {
                    console_error!("Failed to send broadcast message: {}", e);
                    format!("❌ Failed to broadcast message: \"{}\"", message_content)
                } else {
                    format!(
                        "📢 Broadcast message sent to group: \"{}\"",
                        message_content
                    )
                }
            } else {
                format!("❌ Broadcast configuration not found")
            }
        } else if is_broadcast {
            if is_all_plus_message {
                format!(
                    "❌ No group found with last 4 digits: {}",
                    target_group_digits.as_ref().unwrap_or(&"".to_string())
                )
            } else {
                format!("❌ You are not authorized to use @all broadcasts")
            }
        } else {
            create_reply(user_id, message_content)
        }
    }
}

fn create_reply(user_id: &str, message: &str) -> String {
    let lower_message = message.to_lowercase();

    if lower_message.contains("buy") && lower_message.contains("nuclear") {
        return "yes".to_string();
    }

    let user_sum: u32 = user_id.chars().map(|c| c as u32).sum();
    let message_sum: u32 = message.chars().map(|c| c as u32).sum();
    let total_sum = user_sum + message_sum;

    if total_sum % 2 == 0 {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

async fn send_line_reply(reply_token: &str, reply_text: &str, env: &Env) -> Result<()> {
    if reply_token.trim().is_empty() {
        return Err("Reply token cannot be empty".into());
    }

    let channel_access_token = env
        .secret("LINE_CHANNEL_ACCESS_TOKEN")
        .map_err(|_| "LINE_CHANNEL_ACCESS_TOKEN must be set")?
        .to_string();

    let reply_request = ReplyRequest {
        reply_token: reply_token.to_string(),
        messages: vec![ReplyMessage {
            message_type: "text".to_string(),
            text: reply_text.to_string(),
        }],
    };

    let body = serde_json::to_string(&reply_request)?;

    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", channel_access_token))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(body.into()));

    let request = Request::new_with_init("https://api.line.me/v2/bot/message/reply", &init)?;
    let mut response = Fetch::Request(request).send().await?;

    let status = response.status_code();
    if status < 200 || status >= 300 {
        let error_text = response.text().await?;
        return Err(format!("LINE API error: {}", error_text).into());
    }

    Ok(())
}

async fn send_push_message(to: &str, text: &str, env: &Env) -> Result<()> {
    let channel_access_token = env
        .secret("LINE_CHANNEL_ACCESS_TOKEN")
        .map_err(|_| "LINE_CHANNEL_ACCESS_TOKEN must be set")?
        .to_string();

    #[derive(Serialize)]
    struct PushRequest {
        to: String,
        messages: Vec<ReplyMessage>,
    }

    let push_request = PushRequest {
        to: to.to_string(),
        messages: vec![ReplyMessage {
            message_type: "text".to_string(),
            text: text.to_string(),
        }],
    };

    let body = serde_json::to_string(&push_request)?;

    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", channel_access_token))?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(body.into()));

    let request = Request::new_with_init("https://api.line.me/v2/bot/message/push", &init)?;
    let mut response = Fetch::Request(request).send().await?;

    let status = response.status_code();
    if status < 200 || status >= 300 {
        let error_text = response.text().await?;
        return Err(format!("LINE API push error: {}", error_text).into());
    }

    console_log!("Push message sent to {}: {}", to, text);
    Ok(())
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .get("/", |_, _| {
            let health_response = HealthResponse {
                status: "ok".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            Response::from_json(&health_response)
        })
        .get_async("/webhook", |_, _| async move {
            let health_response = HealthResponse {
                status: "ok".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            Response::from_json(&health_response)
        })
        .post_async("/webhook", |mut req, ctx| async move {
            let env = ctx.env;
            let kv = env.kv("DOLPHIN_REPLY_STATE")?;
            let db = env.d1("DOLPHIN_DB")?;

            // Get the raw body for signature verification
            let body_bytes = req.bytes().await?;

            // Skip signature verification in dev mode
            let skip_verification = env
                .var("SKIP_SIGNATURE_VERIFICATION")
                .map(|v| v.to_string() == "true")
                .unwrap_or(false);

            let disable_repeat_detection = env
                .var("DISABLE_REPEAT_DETECTION")
                .map(|v| v.to_string() == "true")
                .unwrap_or(false);

            if !skip_verification {
                // Get the channel secret for signature verification
                let channel_secret = env
                    .secret("LINE_CHANNEL_SECRET")
                    .map_err(|_| "LINE_CHANNEL_SECRET must be set")?
                    .to_string();

                // Verify signature
                let signature_valid = match req.headers().get("x-line-signature") {
                    Ok(Some(sig)) => verify_signature(&body_bytes, &sig, &channel_secret),
                    Ok(None) => {
                        console_error!("Missing signature header");
                        false
                    }
                    Err(e) => {
                        console_error!("Error reading headers: {}", e);
                        false
                    }
                };

                if !signature_valid {
                    console_error!("Invalid or missing signature");
                    return Response::error("Unauthorized", 401);
                }
            } else {
                console_log!("⚠️  Dev mode: Skipping signature verification");
            }

            // Parse webhook request
            let webhook_request: WebhookRequest = match serde_json::from_slice(&body_bytes) {
                Ok(req) => req,
                Err(e) => {
                    console_error!("Failed to parse webhook request: {}", e);
                    return Response::error("Bad Request", 400);
                }
            };

            // Process events
            for event in webhook_request.events {
                if event.delivery_context.is_redelivery {
                    console_log!("Skipping redelivered event: {}", event.webhook_event_id);
                    continue;
                }

                if event.event_type == "message" {
                    if let Some(ref message) = event.message {
                        if message.message_type == "text" {
                            if let Some(text) = &message.text {
                                if let Some(reply_token) = &event.reply_token {
                                    if !reply_token.is_empty() {
                                        if let Err(e) = send_reply(
                                            reply_token,
                                            text,
                                            &event.source,
                                            &env,
                                            &kv,
                                            &db,
                                            disable_repeat_detection,
                                        )
                                        .await
                                        {
                                            console_error!("Failed to send reply: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Response::ok("")
        })
        .run(req, env)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_valid() {
        let body = b"{\"destination\":\"abc\",\"events\":[]}";
        let secret = "channel_secret";

        // Calculate expected signature manually to verify
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        let result = verify_signature(body, &expected, secret);
        assert!(result);
    }

    #[test]
    fn test_verify_signature_invalid() {
        let body = b"{\"destination\":\"abc\",\"events\":[]}";
        let secret = "channel_secret";
        let invalid_signature = "invalid_sig_base64";

        let result = verify_signature(body, invalid_signature, secret);
        assert!(!result);
    }

    fn entry(user_id: &str, message: &str) -> MessageEntry {
        MessageEntry {
            user_id: user_id.to_string(),
            message: message.to_string(),
        }
    }

    #[test]
    fn test_repeated_reply_same_user() {
        let last = entry("U1", "Hello");
        assert_eq!(repeated_reply(&last, "U1", "Hello"), None);
    }

    #[test]
    fn test_repeated_reply_different_user_prefix() {
        let last = entry("U1", "Hello");
        assert_eq!(
            repeated_reply(&last, "U2", "hello world"),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_repeated_reply_mixed_case() {
        let last = entry("U1", "hElLo");
        assert_eq!(
            repeated_reply(&last, "U2", "HELLO"),
            Some("hello".to_string())
        );
    }

    #[test]
    fn test_repeated_reply_no_match() {
        let last = entry("U1", "Hello");
        assert_eq!(repeated_reply(&last, "U2", "Goodbye"), None);
    }
}
