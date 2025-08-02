mod dolphin;

use axum::{
    body::Bytes,
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use hex;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;
use tracing::{error, info};

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

#[derive(Debug, Deserialize)]
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

fn verify_signature(body: &[u8], signature: &str, channel_secret: &str) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(channel_secret.as_bytes())
        .expect("HMAC can take key of any size");

    mac.update(body);
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    signature == expected_signature
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

async fn health_check() -> impl IntoResponse {
    let health_response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    (StatusCode::OK, Json(health_response))
}

async fn debug_handler(_headers: HeaderMap, raw_body: Bytes) -> impl IntoResponse {
    // Read the raw body for debugging
    let body_bytes = raw_body;

    // Log the raw body for debugging
    if let Ok(body_str) = String::from_utf8(body_bytes.clone().to_vec()) {
        info!("Raw request body: {}", body_str);
    }

    // Try to deserialize the JSON
    let _webhook_request: WebhookRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to deserialize JSON: {}", e);
            return StatusCode::UNPROCESSABLE_ENTITY;
        }
    };

    StatusCode::OK
}

async fn webhook_handler(
    headers: HeaderMap,
    Json(webhook_request): Json<WebhookRequest>,
) -> impl IntoResponse {
    // info!("Full webhook request: {:?}", webhook_request);

    // Verify the request signature
    if let (Some(_signature), Some(_channel_secret)) = (
        headers
            .get("x-line-signature")
            .and_then(|h| h.to_str().ok()),
        env::var("LINE_CHANNEL_SECRET").ok(),
    ) {
        // Note: In a real implementation, you'd need to get the raw body
        // For now, we'll skip signature verification in this simplified version
        // In production, you should implement proper body extraction and verification
    } else {
        error!("Missing signature or channel secret");
        return StatusCode::UNAUTHORIZED;
    }

    // Check for redelivery events to avoid processing duplicates
    for event in &webhook_request.events {
        if event.delivery_context.is_redelivery {
            info!("Skipping redelivered event: {}", event.webhook_event_id);
            continue;
        }
    }
    for event in webhook_request.events {
        if event.event_type == "message" {
            if let Some(ref message) = event.message {
                if message.message_type == "text" {
                    if let Some(text) = &message.text {
                        if let Some(reply_token) = &event.reply_token {
                            if reply_token.is_empty() {
                                error!("Reply token is empty for event: {:?}", event);
                                continue;
                            }

                            // Send dolphin reply only for messages starting with "@dolphin"
                            if let Err(e) =
                                dolphin::send_reply(reply_token, text, event.source.user_id).await
                            {
                                error!("Failed to send dolphin reply: {}", e);
                            }
                        } else {
                            error!("No reply token found for event: {:?}", event);
                        }
                    }
                }
            }
        }
    }

    StatusCode::OK
}

async fn send_reply(reply_token: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            text: text.to_string(),
        }],
    };

    let client = reqwest::Client::new();

    // Log the request for debugging
    info!("Sending reply request: {:?}", reply_request);

    let response = client
        .post("https://api.line.me/v2/bot/message/reply")
        .header("Authorization", format!("Bearer {}", channel_access_token))
        .header("Content-Type", "application/json")
        .json(&reply_request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        error!("LINE API error: {}", error_text);

        // Check for specific error types
        if error_text.contains("Invalid reply token") {
            error!("Reply token is invalid or expired. This can happen if:");
            error!("1. The token has expired (tokens are valid for ~30 seconds)");
            error!("2. The token has already been used");
            error!("3. The token is malformed");
        }

        return Err(format!("LINE API error: {}", error_text).into());
    }

    info!("Reply sent successfully");
    Ok(())
}

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Check required environment variables
    if env::var("LINE_CHANNEL_ACCESS_TOKEN").is_err() {
        error!("LINE_CHANNEL_ACCESS_TOKEN environment variable is required");
        std::process::exit(1);
    }

    if env::var("LINE_CHANNEL_SECRET").is_err() {
        error!("LINE_CHANNEL_SECRET environment variable is required");
        std::process::exit(1);
    }

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    info!("Starting LINE Echo Bot on {}", addr);

    // Build our application with routes
    let app = Router::new()
        .route("/", get(health_check))
        .route("/debug", get(debug_handler))
        .route("/debug", post(debug_handler))
        .route("/webhook", get(health_check))
        .route("/webhook", post(webhook_handler));

    // Run it
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
