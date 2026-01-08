use std::collections::{HashMap, VecDeque};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, warn};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub payload: String, // Keep text for simplicity in MVP, or Vec<u8> with base64
    pub timestamp: u64,
    #[serde(default)]
    pub priority: Option<MessagePriority>,
}

impl Message {
    pub fn new(payload: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: now_ms_safe(),
            priority: None, // Default to Normal (None = Normal)
        }
    }
    
    pub fn with_priority(payload: String, priority: Option<MessagePriority>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: now_ms_safe(),
            priority,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub broadcast_channel_capacity: usize,
    pub max_expired_events: usize,
    pub topic_replay_count: usize,
    pub inactivity_timeout_seconds: Option<u64>,
    pub topic_exclude_patterns: Vec<String>,
    /// Maximum payload size in bytes (0 = unlimited)
    pub max_payload_size: usize,
    /// Maximum topic/queue name length (0 = unlimited)
    pub max_name_length: usize,
    /// Maximum number of topics (0 = unlimited)
    pub max_topics: usize,
    /// Maximum number of queues (0 = unlimited)
    pub max_queues: usize,
    /// Maximum queue depth (0 = unlimited)
    pub max_queue_depth: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            broadcast_channel_capacity: 1024,
            max_expired_events: 50,
            topic_replay_count: 0,
            inactivity_timeout_seconds: None,
            topic_exclude_patterns: Vec::new(),
            max_payload_size: 10 * 1024 * 1024, // 10MB default
            max_name_length: 256, // 256 chars default
            max_topics: 0, // Unlimited by default
            max_queues: 0, // Unlimited by default
            max_queue_depth: 0, // Unlimited by default
        }
    }
}

pub struct Engine {
    pub config: EngineConfig,
    // Pub/Sub: simple broadcast
    pub topics: RwLock<HashMap<String, TopicEntry>>,
    // Queue: Point-to-Point
    pub queues: RwLock<HashMap<String, QueueEntry>>,
    expired_events: RwLock<Vec<ExpiredEntry>>,
    // Pattern subscriptions: pattern -> list of matching topics
    pattern_subscriptions: RwLock<HashMap<String, Vec<String>>>,
}

#[derive(Clone)]
pub struct TopicEntry {
    pub sender: broadcast::Sender<Message>,
    pub meta: EntryMeta,
    pub message_history: VecDeque<Message>,
}

#[derive(Clone)]
pub struct PriorityQueues {
    pub high: VecDeque<Message>,
    pub normal: VecDeque<Message>,
    pub low: VecDeque<Message>,
}

impl PriorityQueues {
    fn new() -> Self {
        Self {
            high: VecDeque::new(),
            normal: VecDeque::new(),
            low: VecDeque::new(),
        }
    }
    
    fn total_len(&self) -> usize {
        self.high.len() + self.normal.len() + self.low.len()
    }
    
    fn push(&mut self, msg: Message) {
        let priority = msg.priority.as_ref().unwrap_or(&MessagePriority::Normal);
        match priority {
            MessagePriority::High => self.high.push_back(msg),
            MessagePriority::Normal => self.normal.push_back(msg),
            MessagePriority::Low => self.low.push_back(msg),
        }
    }
    
    fn pop(&mut self) -> Option<Message> {
        if let Some(msg) = self.high.pop_front() {
            return Some(msg);
        }
        if let Some(msg) = self.normal.pop_front() {
            return Some(msg);
        }
        self.low.pop_front()
    }
}

#[derive(Clone)]
pub struct QueueEntry {
    pub messages: PriorityQueues,
    pub meta: EntryMeta,
}

#[derive(Clone, Debug, Serialize)]
pub struct EntryMeta {
    pub ttl_seconds: Option<u64>,
    pub created_at: u64,
    pub last_activity: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TopicStatus {
    pub subscribers: usize,
    pub meta: EntryMeta,
    pub expires_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueueStatus {
    pub depth: usize,
    pub meta: EntryMeta,
    pub expires_at: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExpiredEntry {
    pub name: String,
    pub kind: String,
    pub expired_at: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BrokerSnapshot {
    pub topics: HashMap<String, TopicStatus>,
    pub queues: HashMap<String, QueueStatus>,
    pub expired: Vec<ExpiredEntry>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            topics: RwLock::new(HashMap::new()),
            queues: RwLock::new(HashMap::new()),
            expired_events: RwLock::new(Vec::new()),
            pattern_subscriptions: RwLock::new(HashMap::new()),
        }
    }

    /// Validate topic/queue name
    pub fn validate_name(&self, name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        if self.config.max_name_length > 0 && name.len() > self.config.max_name_length {
            return Err(format!("Name too long (max {} chars)", self.config.max_name_length));
        }
        Ok(())
    }

    /// Validate message payload size
    pub fn validate_payload_size(&self, payload: &str) -> Result<(), String> {
        if self.config.max_payload_size > 0 && payload.len() > self.config.max_payload_size {
            return Err(format!("Payload too large (max {} bytes)", self.config.max_payload_size));
        }
        Ok(())
    }

    pub fn publish(&self, topic: &str, msg: Message, ttl_seconds: Option<u64>) -> Result<usize, String> {
        // Validate topic name
        self.validate_name(topic)?;
        
        // Validate payload size
        self.validate_payload_size(&msg.payload)?;

        let mut topics = self.topics.write();
        
        // Check topic limit
        if self.config.max_topics > 0 && !topics.contains_key(topic) && topics.len() >= self.config.max_topics {
            return Err(format!("Maximum number of topics ({}) reached", self.config.max_topics));
        }
        
        let is_new_topic = !topics.contains_key(topic);
        
        if let Some(entry) = topics.get_mut(topic) {
            entry.meta.touch();
            // Save to history if replay is enabled
            if self.config.topic_replay_count > 0 {
                entry.message_history.push_back(msg.clone());
                // Limit history size
                while entry.message_history.len() > self.config.topic_replay_count {
                    entry.message_history.pop_front();
                }
            }
            // Try to send, handle channel overflow gracefully
            let result = match entry.sender.send(msg) {
                Ok(count) => Ok(count),
                Err(broadcast::error::SendError(_)) => {
                    warn!("Broadcast channel full for topic '{}', message dropped", topic);
                    Ok(0) // Return 0 subscribers if channel is full
                }
            };
            // Release lock before pattern check
            drop(topics);
            result
        } else {
            let (tx, _rx) = broadcast::channel(self.config.broadcast_channel_capacity);
            let count = match tx.send(msg.clone()) {
                Ok(count) => count,
                Err(broadcast::error::SendError(_)) => {
                    warn!("Failed to send initial message to new topic '{}'", topic);
                    0
                }
            };
            let mut history = VecDeque::new();
            // Save to history if replay is enabled
            if self.config.topic_replay_count > 0 {
                history.push_back(msg);
            }
            topics.insert(
                topic.to_string(),
                TopicEntry {
                    sender: tx,
                    meta: EntryMeta::new(ttl_seconds),
                    message_history: history,
                },
            );
            
            // Release lock before pattern check
            drop(topics);
            
            // If this is a new topic, check if it matches any active pattern subscriptions
            if is_new_topic {
                let patterns = self.pattern_subscriptions.read();
                for (pattern, _) in patterns.iter() {
                    if matches_pattern(topic, pattern) {
                        // Topic matches a pattern, but subscribers will get it via their existing receivers
                        // No action needed as broadcast channels handle this automatically
                    }
                }
            }
            
            Ok(count)
        }
    }
    
    pub fn subscribe(&self, topic: &str, ttl_seconds: Option<u64>) -> Result<(broadcast::Receiver<Message>, Vec<Message>), String> {
        // Validate topic name
        self.validate_name(topic)?;

        let mut topics = self.topics.write();
        let entry = topics.entry(topic.to_string()).or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(self.config.broadcast_channel_capacity);
            TopicEntry {
                sender: tx,
                meta: EntryMeta::new(ttl_seconds),
                message_history: VecDeque::new(),
            }
        });
        entry.meta.touch();
        let history: Vec<Message> = entry.message_history.iter().cloned().collect();
        Ok((entry.sender.subscribe(), history))
    }

    pub fn push_queue(&self, queue: &str, msg: Message, ttl_seconds: Option<u64>) -> Result<(), String> {
        // Validate queue name
        self.validate_name(queue)?;
        
        // Validate payload size
        self.validate_payload_size(&msg.payload)?;

        let mut queues = self.queues.write();
        
        // Check queue limit
        if self.config.max_queues > 0 && !queues.contains_key(queue) && queues.len() >= self.config.max_queues {
            return Err(format!("Maximum number of queues ({}) reached", self.config.max_queues));
        }
        
        let entry = queues.entry(queue.to_string()).or_insert_with(|| QueueEntry {
            messages: PriorityQueues::new(),
            meta: EntryMeta::new(ttl_seconds),
        });
        
        // Check queue depth limit
        if self.config.max_queue_depth > 0 && entry.messages.total_len() >= self.config.max_queue_depth {
            return Err(format!("Queue '{}' is full (max depth: {})", queue, self.config.max_queue_depth));
        }
        
        entry.meta.touch();
        entry.messages.push(msg);
        Ok(())
    }
    
    pub fn pop_queue(&self, queue: &str) -> Option<Message> {
        // Validate queue name (silently fail for invalid names)
        if self.validate_name(queue).is_err() {
            return None;
        }

        let mut queues = self.queues.write();
        queues.get_mut(queue).and_then(|entry| {
            entry.meta.touch();
            entry.messages.pop()
        })
    }

    pub fn get_stats(&self) -> BrokerSnapshot {
        let topics = self.topics.read();
        let queues = self.queues.read();
        let expired = self.expired_events.read();
        
        let topic_stats: HashMap<_, _> = topics
            .iter()
            .map(|(k, v)| {
                (k.clone(), TopicStatus::new(v.sender.receiver_count(), &v.meta))
            })
            .collect();
        let queue_stats: HashMap<_, _> = queues
            .iter()
            .map(|(k, v)| (k.clone(), QueueStatus::new(v.messages.total_len(), &v.meta)))
            .collect();
        
        BrokerSnapshot {
            topics: topic_stats,
            queues: queue_stats,
            expired: expired.clone(),
        }
    }

    pub fn sweep_expired(&self) {
        let now = now_ms();
        let mut expired_entries = Vec::new();

        {
            let mut topics = self.topics.write();
            let expired_topics: Vec<(String, String)> = topics
                .iter()
                .filter_map(|(name, entry)| {
                    let reason = if entry.meta.is_expired(now) {
                        Some("ttl".to_string())
                    } else if let Some(timeout) = self.config.inactivity_timeout_seconds {
                        let inactive_ms = now.saturating_sub(entry.meta.last_activity);
                        if inactive_ms >= timeout.saturating_mul(1000) {
                            Some("inactivity".to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    reason.map(|r| (name.clone(), r))
                })
                .collect();
            for (name, reason) in expired_topics {
                topics.remove(&name);
                expired_entries.push(ExpiredEntry {
                    name,
                    kind: "topic".to_string(),
                    expired_at: now,
                    reason,
                });
            }
        }

        {
            let mut queues = self.queues.write();
            let expired_queues: Vec<(String, String)> = queues
                .iter()
                .filter_map(|(name, entry)| {
                    let reason = if entry.meta.is_expired(now) {
                        Some("ttl".to_string())
                    } else if let Some(timeout) = self.config.inactivity_timeout_seconds {
                        let inactive_ms = now.saturating_sub(entry.meta.last_activity);
                        if inactive_ms >= timeout.saturating_mul(1000) {
                            Some("inactivity".to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    reason.map(|r| (name.clone(), r))
                })
                .collect();
            for (name, reason) in expired_queues {
                queues.remove(&name);
                expired_entries.push(ExpiredEntry {
                    name,
                    kind: "queue".to_string(),
                    expired_at: now,
                    reason,
                });
            }
        }

        if !expired_entries.is_empty() {
            let mut expired = self.expired_events.write();
            expired.extend(expired_entries);
            if expired.len() > self.config.max_expired_events {
                let drain_count = expired.len() - self.config.max_expired_events;
                expired.drain(0..drain_count);
            }
        }
    }

    pub fn get_metrics(&self) -> BrokerSnapshot {
        self.get_stats()
    }

    /// Check if a topic is excluded from pattern matching
    fn is_topic_excluded(&self, topic: &str) -> bool {
        for exclude_pattern in &self.config.topic_exclude_patterns {
            if matches_pattern(topic, exclude_pattern) {
                return true;
            }
        }
        false
    }

    /// Find all topics matching a pattern (excluding topics that match exclude patterns)
    pub fn find_matching_topics(&self, pattern: &str) -> Vec<String> {
        let topics = self.topics.read();
        topics
            .keys()
            .filter(|topic| {
                // Must match the pattern
                matches_pattern(topic, pattern) &&
                // And must not be excluded
                !self.is_topic_excluded(topic)
            })
            .cloned()
            .collect()
    }
    
    /// Subscribe to a pattern and return receivers for all matching topics
    pub fn subscribe_pattern(&self, pattern: &str, ttl_seconds: Option<u64>) -> Result<Vec<(String, broadcast::Receiver<Message>, Vec<Message>)>, String> {
        // Validate pattern
        if pattern.is_empty() {
            return Err("Pattern cannot be empty".to_string());
        }
        if self.config.max_name_length > 0 && pattern.len() > self.config.max_name_length {
            return Err(format!("Pattern too long (max {} chars)", self.config.max_name_length));
        }

        let matching_topics = self.find_matching_topics(pattern);
        let mut subscriptions = Vec::new();
        
        // Subscribe to all matching topics, skip errors for individual topics
        for topic in &matching_topics {
            match self.subscribe(topic, ttl_seconds) {
                Ok((rx, history)) => {
                    subscriptions.push((topic.clone(), rx, history));
                }
                Err(e) => {
                    warn!("Failed to subscribe to topic '{}' in pattern '{}': {}", topic, pattern, e);
                    // Continue with other topics
                }
            }
        }
        
        // Track this pattern subscription (use the same list we already computed)
        {
            let mut patterns = self.pattern_subscriptions.write();
            patterns.insert(pattern.to_string(), matching_topics);
            
            // Limit pattern subscriptions to prevent memory leaks
            const MAX_PATTERN_SUBSCRIPTIONS: usize = 1000;
            if patterns.len() > MAX_PATTERN_SUBSCRIPTIONS {
                warn!("Too many pattern subscriptions ({}), removing oldest", patterns.len());
                // Remove oldest entries (HashMap doesn't preserve order, so we remove arbitrary ones)
                let keys_to_remove: Vec<String> = patterns.keys().take(patterns.len() - MAX_PATTERN_SUBSCRIPTIONS).cloned().collect();
                for key in keys_to_remove {
                    patterns.remove(&key);
                }
            }
        }
        
        Ok(subscriptions)
    }
}

impl EntryMeta {
    fn new(ttl_seconds: Option<u64>) -> Self {
        let now = now_ms();
        Self {
            ttl_seconds,
            created_at: now,
            last_activity: now,
        }
    }

    fn touch(&mut self) {
        self.last_activity = now_ms();
    }

    fn is_expired(&self, now: u64) -> bool {
        self.ttl_seconds
            .map(|ttl| now.saturating_sub(self.last_activity) >= ttl.saturating_mul(1000))
            .unwrap_or(false)
    }

    fn expires_at(&self) -> Option<u64> {
        self.ttl_seconds
            .map(|ttl| self.last_activity.saturating_add(ttl.saturating_mul(1000)))
    }
}

impl TopicStatus {
    fn new(subscribers: usize, meta: &EntryMeta) -> Self {
        Self {
            subscribers,
            meta: meta.clone(),
            expires_at: meta.expires_at(),
        }
    }
}

impl QueueStatus {
    fn new(depth: usize, meta: &EntryMeta) -> Self {
        Self {
            depth,
            meta: meta.clone(),
            expires_at: meta.expires_at(),
        }
    }
}

/// Safe version of now_ms that handles clock errors gracefully
fn now_ms_safe() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_else(|_| {
            // If system time is before epoch, log error and return 0
            error!("System time is before UNIX epoch, using 0 as timestamp");
            0
        })
}

fn now_ms() -> u64 {
    now_ms_safe()
}

/// Check if a topic name matches a pattern
/// Supports:
/// - `*` matches a single segment (e.g., `sensor.*` matches `sensor.temp` but not `sensor.room.temp`)
/// - `**` matches any path (e.g., `logs.**` matches `logs.app.error`, `logs.system.info`)
pub fn matches_pattern(topic: &str, pattern: &str) -> bool {
    if pattern == topic {
        return true;
    }
    
    // Handle ** (match any path)
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            if prefix.is_empty() && suffix.is_empty() {
                return true; // ** matches everything
            }
            if prefix.is_empty() {
                return topic.ends_with(suffix);
            }
            if suffix.is_empty() {
                return topic.starts_with(prefix);
            }
            return topic.starts_with(prefix) && topic.ends_with(suffix);
        }
    }
    
    // Handle * (match single segment)
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let topic_parts: Vec<&str> = topic.split('.').collect();
    
    if pattern_parts.len() != topic_parts.len() {
        return false;
    }
    
    for (pattern_part, topic_part) in pattern_parts.iter().zip(topic_parts.iter()) {
        if *pattern_part != "*" && pattern_part != topic_part {
            return false;
        }
    }
    
    true
}
