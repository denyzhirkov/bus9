use std::collections::{HashMap, VecDeque};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub payload: String, // Keep text for simplicity in MVP, or Vec<u8> with base64
    pub timestamp: u64,
}

impl Message {
    pub fn new(payload: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
        }
    }
}

pub struct Engine {
    // Pub/Sub: simple broadcast
    pub topics: RwLock<HashMap<String, TopicEntry>>,
    // Queue: Point-to-Point
    pub queues: RwLock<HashMap<String, QueueEntry>>,
    expired_events: RwLock<Vec<ExpiredEntry>>,
}

#[derive(Clone)]
pub struct TopicEntry {
    pub sender: broadcast::Sender<Message>,
    pub meta: EntryMeta,
}

#[derive(Clone)]
pub struct QueueEntry {
    pub messages: VecDeque<Message>,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct BrokerSnapshot {
    pub topics: HashMap<String, TopicStatus>,
    pub queues: HashMap<String, QueueStatus>,
    pub expired: Vec<ExpiredEntry>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            topics: RwLock::new(HashMap::new()),
            queues: RwLock::new(HashMap::new()),
            expired_events: RwLock::new(Vec::new()),
        }
    }

    pub fn publish(&self, topic: &str, msg: Message, ttl_seconds: Option<u64>) -> usize {
        let mut topics = self.topics.write();
        if let Some(entry) = topics.get_mut(topic) {
            entry.meta.touch();
            entry.sender.send(msg).unwrap_or(0)
        } else {
            let (tx, _rx) = broadcast::channel(1024);
            let count = tx.send(msg).unwrap_or(0);
            topics.insert(
                topic.to_string(),
                TopicEntry {
                    sender: tx,
                    meta: EntryMeta::new(ttl_seconds),
                },
            );
            count
        }
    }
    
    pub fn subscribe(&self, topic: &str, ttl_seconds: Option<u64>) -> broadcast::Receiver<Message> {
        let mut topics = self.topics.write();
        let entry = topics.entry(topic.to_string()).or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(1024);
            TopicEntry {
                sender: tx,
                meta: EntryMeta::new(ttl_seconds),
            }
        });
        entry.meta.touch();
        entry.sender.subscribe()
    }

    pub fn push_queue(&self, queue: &str, msg: Message, ttl_seconds: Option<u64>) {
        let mut queues = self.queues.write();
        let entry = queues.entry(queue.to_string()).or_insert_with(|| QueueEntry {
            messages: VecDeque::new(),
            meta: EntryMeta::new(ttl_seconds),
        });
        entry.meta.touch();
        entry.messages.push_back(msg);
    }
    
    pub fn pop_queue(&self, queue: &str) -> Option<Message> {
        let mut queues = self.queues.write();
        queues.get_mut(queue).and_then(|entry| {
            entry.meta.touch();
            entry.messages.pop_front()
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
            .map(|(k, v)| (k.clone(), QueueStatus::new(v.messages.len(), &v.meta)))
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
            let expired_topics: Vec<String> = topics
                .iter()
                .filter(|(_, entry)| entry.meta.is_expired(now))
                .map(|(name, _)| name.clone())
                .collect();
            for name in expired_topics {
                topics.remove(&name);
                expired_entries.push(ExpiredEntry {
                    name,
                    kind: "topic".to_string(),
                    expired_at: now,
                });
            }
        }

        {
            let mut queues = self.queues.write();
            let expired_queues: Vec<String> = queues
                .iter()
                .filter(|(_, entry)| entry.meta.is_expired(now))
                .map(|(name, _)| name.clone())
                .collect();
            for name in expired_queues {
                queues.remove(&name);
                expired_entries.push(ExpiredEntry {
                    name,
                    kind: "queue".to_string(),
                    expired_at: now,
                });
            }
        }

        if !expired_entries.is_empty() {
            let mut expired = self.expired_events.write();
            expired.extend(expired_entries);
            if expired.len() > 50 {
                let drain_count = expired.len() - 50;
                expired.drain(0..drain_count);
            }
        }
    }

    pub fn get_metrics(&self) -> BrokerSnapshot {
        self.get_stats()
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
