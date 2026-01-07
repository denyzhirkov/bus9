use std::collections::{HashMap, VecDeque};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            priority: None, // Default to Normal (None = Normal)
        }
    }
    
    pub fn with_priority(payload: String, priority: Option<MessagePriority>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
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
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            broadcast_channel_capacity: 1024,
            max_expired_events: 50,
            topic_replay_count: 0,
            inactivity_timeout_seconds: None,
            topic_exclude_patterns: Vec::new(),
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
    
    /// Get a reference to the engine config
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn publish(&self, topic: &str, msg: Message, ttl_seconds: Option<u64>) -> usize {
        let mut topics = self.topics.write();
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
            entry.sender.send(msg).unwrap_or(0)
        } else {
            let (tx, _rx) = broadcast::channel(self.config.broadcast_channel_capacity);
            let count = tx.send(msg.clone()).unwrap_or(0);
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
            
            // If this is a new topic, check if it matches any active pattern subscriptions
            if is_new_topic {
                drop(topics); // Release write lock
                let patterns = self.pattern_subscriptions.read();
                for (pattern, _) in patterns.iter() {
                    if matches_pattern(topic, pattern) {
                        // Topic matches a pattern, but subscribers will get it via their existing receivers
                        // No action needed as broadcast channels handle this automatically
                    }
                }
            }
            
            count
        }
    }
    
    pub fn subscribe(&self, topic: &str, ttl_seconds: Option<u64>) -> (broadcast::Receiver<Message>, Vec<Message>) {
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
        (entry.sender.subscribe(), history)
    }

    pub fn push_queue(&self, queue: &str, msg: Message, ttl_seconds: Option<u64>) {
        let mut queues = self.queues.write();
        let entry = queues.entry(queue.to_string()).or_insert_with(|| QueueEntry {
            messages: PriorityQueues::new(),
            meta: EntryMeta::new(ttl_seconds),
        });
        entry.meta.touch();
        entry.messages.push(msg);
    }
    
    pub fn pop_queue(&self, queue: &str) -> Option<Message> {
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
    pub fn subscribe_pattern(&self, pattern: &str, ttl_seconds: Option<u64>) -> Vec<(String, broadcast::Receiver<Message>, Vec<Message>)> {
        let matching_topics = self.find_matching_topics(pattern);
        let mut subscriptions = Vec::new();
        
        for topic in matching_topics {
            let (rx, history) = self.subscribe(&topic, ttl_seconds);
            subscriptions.push((topic, rx, history));
        }
        
        // Track this pattern subscription
        {
            let mut patterns = self.pattern_subscriptions.write();
            patterns.insert(pattern.to_string(), self.find_matching_topics(pattern));
        }
        
        subscriptions
    }
    
    /// Check if a string is a pattern (contains * or **)
    pub fn is_pattern(&self, s: &str) -> bool {
        s.contains('*')
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
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
