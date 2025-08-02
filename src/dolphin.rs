use serde::Serialize;
use std::env;
use tracing::{error, info};

#[derive(Debug, Serialize)]
struct ReplyMessage {
    #[serde(rename = "type")]
    message_type: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct ReplyRequest {
    #[serde(rename = "replyToken")]
    reply_token: String,
    messages: Vec<ReplyMessage>,
}

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

pub async fn send_reply(
    reply_token: &str,
    text: &str,
    user_id: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if the message starts with "@dolphin"
    if !text.trim().to_lowercase().starts_with("@dolphin") {
        return Ok(());
    }

    // Extract the message content after "@dolphin"
    let message_content = text.trim()[8..].trim(); // Remove "@dolphin" prefix

    if message_content.is_empty() {
        info!("Message only contains '@dolphin' with no additional content");
        return Ok(());
    }

    // Validate reply token
    if reply_token.trim().is_empty() {
        return Err("Reply token cannot be empty".into());
    }

    // Get user ID for checksum calculation
    let user_id = user_id.unwrap_or_else(|| "unknown".to_string());

    // Create reply using checksum logic
    let reply_text = create_reply(&user_id, message_content);

    let channel_access_token =
        env::var("LINE_CHANNEL_ACCESS_TOKEN").expect("LINE_CHANNEL_ACCESS_TOKEN must be set");

    let reply_request = ReplyRequest {
        reply_token: reply_token.to_string(),
        messages: vec![ReplyMessage {
            message_type: "text".to_string(),
            text: format!("{}", reply_text),
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
        return Err(format!("LINE API error: {}\n\t{}", error_text, message_content).into());
    }

    info!("Reply sent: {}", message_content);
    Ok(())
}
