use crate::engine::MessagePriority;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PayloadRequest {
    pub payload: String,
    pub ttl_seconds: Option<u64>,
    pub priority: Option<String>,
}

pub fn parse_payload(body: String) -> (String, Option<u64>, Option<String>) {
    match serde_json::from_str::<PayloadRequest>(&body) {
        Ok(request) => (request.payload, request.ttl_seconds, request.priority),
        Err(_) => (body, None, None),
    }
}

pub fn parse_priority(priority_str: Option<&String>) -> Option<MessagePriority> {
    priority_str.as_ref().and_then(|p| match p.to_lowercase().as_str() {
        "high" => Some(MessagePriority::High),
        "normal" => Some(MessagePriority::Normal),
        "low" => Some(MessagePriority::Low),
        _ => None,
    })
}
