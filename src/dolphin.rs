use crate::types::{ReplyMessage, ReplyRequest};
use std::env;
use std::fs;
use tracing::{error, info};

#[derive(Debug)]
struct BroadcastConfig {
    allowed_user_id: String,
    target_group_id: String,
}

impl BroadcastConfig {
    fn from_env() -> Vec<Self> {
        let mut configs = Vec::new();

        // Check for numbered configurations (DOLPHIN_USER_TO_GROUP1, DOLPHIN_USER_TO_GROUP2, etc.)
        for i in 1..=10 {  // Support up to 10 configurations
            let env_key = format!("DOLPHIN_USER_TO_GROUP{}", i);
            if let Ok(var) = env::var(&env_key) {
                if let Some(config) = Self::parse_config(&var) {
                    configs.push(config);
                }
            }
        }

        // Also check for the original DOLPHIN_USER_TO_GROUP (for backward compatibility)
        if let Ok(var) = env::var("DOLPHIN_USER_TO_GROUP") {
            if let Some(config) = Self::parse_config(&var) {
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
        configs.iter().find(|config| config.is_user_authorized(user_id))
    }

    fn has_authorized_user(configs: &[Self]) -> bool {
        !configs.is_empty()
    }
}

fn get_reply_state_file() -> String {
    env::var("REPLY_STATE_FILE").unwrap_or_else(|_| "reply_state.txt".to_string())
}

fn is_replies_enabled() -> bool {
    let state_file = get_reply_state_file();
    if let Ok(content) = fs::read_to_string(&state_file) {
        content.trim() == "enabled"
    } else {
        // Default to enabled if file doesn't exist or can't be read
        true
    }
}

fn set_replies_enabled(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let state_file = get_reply_state_file();
    let state = if enabled { "enabled" } else { "disabled" };
    fs::write(&state_file, state)?;
    Ok(())
}

pub async fn send_reply(
    reply_token: &str,
    text: &str,
    source: &crate::Source,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get user ID for checksum calculation
    let user_id = source.user_id.as_deref().unwrap_or("unknown");

    // Check if source has group-id
    let has_group_id = source.group_id.is_some();

    // For group messages, check if the message starts with "@dolphin" or "@all"
    let trimmed_text = text.trim().to_lowercase();
    let is_dolphin_message = trimmed_text.starts_with("@dolphin");
    let is_all_message = trimmed_text.starts_with("@all");
    let is_off_command = trimmed_text.starts_with("@off");
    let is_on_command = trimmed_text.starts_with("@on");

    // Handle @off and @on commands from authorized user
    if is_off_command || is_on_command {
        let broadcast_configs = BroadcastConfig::from_env();
        if BroadcastConfig::has_authorized_user(&broadcast_configs) {
            if BroadcastConfig::find_by_user_id(&broadcast_configs, user_id).is_some() {
                let enable = is_on_command;
                if set_replies_enabled(enable).is_ok() {
                    let status = if enable { "enabled" } else { "disabled" };
                    let reply_text = format!("🔧 Replies have been {}", status);
                    send_line_reply(reply_token, &reply_text).await?;
                    info!("Reply status changed to {} by user {}", status, user_id);
                    return Ok(());
                } else {
                    error!("Failed to save reply state");
                    let reply_text = "❌ Failed to change reply status".to_string();
                    send_line_reply(reply_token, &reply_text).await?;
                    return Ok(());
                }
            } else {
                let reply_text = "❌ You are not authorized to control reply settings".to_string();
                send_line_reply(reply_token, &reply_text).await?;
                info!("Unauthorized attempt to control replies by user {}", user_id);
                return Ok(());
            }
        } else {
            let reply_text = "❌ Broadcast configuration not found".to_string();
            send_line_reply(reply_token, &reply_text).await?;
            return Ok(());
        }
    }

    // Check if replies are enabled
    if !is_replies_enabled() {
        info!(
            "Replies are disabled, ignoring message from user {}",
            user_id
        );
        return Ok(());
    }

    if !is_dolphin_message && !is_all_message {
        if has_group_id {
            // Ignore messages that don't start with "@dolphin" or "@all" in group chats
            return Ok(());
        }

        // For direct messages, reply to all messages
        let reply_text = create_reply(user_id, text);
        send_line_reply(reply_token, &reply_text).await?;
        info!("Reply sent (user_id: {}): {}", user_id, text);
        return Ok(());
    }

    // Get broadcast configs once to avoid repeated parsing
    let broadcast_configs = BroadcastConfig::from_env();
    let (message_content, is_broadcast, authorized_broadcast) = if is_all_message {
        // Extract the message content after "@all"
        let content = text.trim()[4..].trim(); // Remove "@all" prefix

        // Check if user is authorized to broadcast
        let authorized = BroadcastConfig::find_by_user_id(&broadcast_configs, user_id).is_some();

        (content, true, authorized)
    } else {
        // Extract the message content after "@dolphin"
        let content = text.trim()[8..].trim(); // Remove "@dolphin" prefix
        (content, false, false)
    };

    if message_content.is_empty() {
        return Ok(());
    }

    // Create reply based on message type
    let group_id = source.group_id.as_deref().unwrap_or("unknown");
    let reply_text = if has_group_id {
        // For @dolphin and @all messages, use the standard checksum logic
        create_reply(user_id, message_content)
    } else {
        if authorized_broadcast {
            // Send broadcast message to target group
            if let Some(config) = BroadcastConfig::find_by_user_id(&broadcast_configs, user_id) {
                if let Err(e) = send_push_message(&config.target_group_id, message_content).await {
                    error!("Failed to send broadcast message: {}", e);
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
            // User not authorized to broadcast
            format!("❌ You are not authorized to use @all broadcasts")
        } else {
            create_reply(user_id, message_content)
        }
    };

    send_line_reply(reply_token, &reply_text).await?;
    info!(
        "Reply sent (group_id:{}, broadcast:{}, user_id:{}): {}",
        group_id,
        is_broadcast,
        &user_id[0..4],
        message_content
    );
    Ok(())
}

/// Creates a reply based on checksum of user ID and message
/// Returns "yes" if the sum is even, "no" if odd
/// Special case: if message contains both "buy" and "nuclear", always returns "yes"
fn create_reply(user_id: &str, message: &str) -> String {
    let lower_message = message.to_lowercase();

    // Special case: if message contains both "buy" and "nuclear", always return "yes"
    if lower_message.contains("buy") && lower_message.contains("nuclear") {
        return "yes".to_string();
    }

    // Calculate checksum: sum of all ASCII values in user_id + message
    let user_sum: u32 = user_id.chars().map(|c| c as u32).sum();
    let message_sum: u32 = message.chars().map(|c| c as u32).sum();
    let total_sum = user_sum + message_sum;

    // Return "yes" if even, "no" if odd
    if total_sum % 2 == 0 {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

async fn send_line_reply(
    reply_token: &str,
    reply_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate reply token
    if reply_token.trim().is_empty() {
        return Err("Reply token cannot be empty".into());
    }

    let channel_access_token =
        env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("LINE_CHANNEL_ACCESS_TOKEN must be set");

    let reply_request = ReplyRequest {
        reply_token: reply_token.to_string(),
        messages: vec![ReplyMessage {
            message_type: "text".to_string(),
            text: reply_text.to_string(),
        }],
    };

    let client = reqwest::Client::new();

    let response = client
        .post("https://api.line.me/v2/bot/message/reply")
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .header("Content-Type", "application/json")
        .json(&reply_request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("LINE API error: {}", error_text).into());
    }

    Ok(())
}

async fn send_push_message(to: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let channel_access_token =
        env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("LINE_CHANNEL_ACCESS_TOKEN must be set");

    #[derive(serde::Serialize)]
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

    let client = reqwest::Client::new();

    let response = client
        .post("https://api.line.me/v2/bot/message/push")
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .header("Content-Type", "application/json")
        .json(&push_request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("LINE API push error: {}", error_text).into());
    }

    info!("Push message sent to {}: {}", to, text);
    Ok(())
}
