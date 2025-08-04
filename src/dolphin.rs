use crate::types::{ReplyMessage, ReplyRequest};
use std::env;
use tracing::info;

/// Creates a reply based on checksum of user ID and message
/// Returns "yes" if the sum is even, "no" if odd
/// Special case: if message contains both "buy" and "nuclear", always returns "yes"
pub fn create_reply(user_id: &str, message: &str) -> String {
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

pub async fn send_reply(
    reply_token: &str,
    text: &str,
    source: &crate::Source,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get user ID for checksum calculation
    let user_id = source.user_id.as_deref().unwrap_or("unknown");

    // Check if source has group-id
    let has_group_id = source.group_id.is_some();

    // Direct message from user
    if !has_group_id {
        // Create reply using checksum logic for the entire message
        let reply_text = create_reply(user_id, text);
        send_line_reply(reply_token, &reply_text).await?;
        info!("Reply sent (user_id: {}): {}", user_id, text);
        return Ok(());
    }

    // For group messages, check if the message starts with "@dolphin"
    if !text.trim().to_lowercase().starts_with("@dolphin") {
        return Ok(());
    }

    // Extract the message content after "@dolphin"
    let text = text.trim()[8..].trim(); // Remove "@dolphin" prefix
    if text.is_empty() {
        info!("Message only contains '@dolphin' with no additional content");
        return Ok(());
    }

    // Create reply using checksum logic
    let group_id = source.group_id.as_deref().unwrap_or("unknown");
    let reply_text = create_reply(user_id, text);
    send_line_reply(reply_token, &reply_text).await?;
    info!("Reply sent (group_id: {}): {}", group_id, text);
    Ok(())
}
