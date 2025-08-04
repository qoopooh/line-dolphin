use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReplyMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct ReplyRequest {
    #[serde(rename = "replyToken")]
    pub reply_token: String,
    pub messages: Vec<ReplyMessage>,
}
