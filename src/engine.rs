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
    pub topics: RwLock<HashMap<String, broadcast::Sender<Message>>>,
    // Queue: Point-to-Point
    pub queues: RwLock<HashMap<String, VecDeque<Message>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            topics: RwLock::new(HashMap::new()),
            queues: RwLock::new(HashMap::new()),
        }
    }

    pub fn publish(&self, topic: &str, msg: Message) -> usize {
        let mut topics = self.topics.write();
        if let Some(sender) = topics.get(topic) {
             sender.send(msg).unwrap_or(0)
        } else {
            let (tx, _rx) = broadcast::channel(1024);
            let count = tx.send(msg).unwrap_or(0);
            topics.insert(topic.to_string(), tx);
            count
        }
    }
    
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<Message> {
        let mut topics = self.topics.write();
        let tx = topics.entry(topic.to_string()).or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(1024);
            tx
        });
        tx.subscribe()
    }

    pub fn push_queue(&self, queue: &str, msg: Message) {
        let mut queues = self.queues.write();
        queues.entry(queue.to_string()).or_default().push_back(msg);
    }
    
    pub fn pop_queue(&self, queue: &str) -> Option<Message> {
        let mut queues = self.queues.write();
        queues.get_mut(queue).and_then(|q| q.pop_front())
    }

    pub fn get_stats(&self) -> serde_json::Value {
        let topics = self.topics.read();
        let queues = self.queues.read();
        
        let topic_stats: HashMap<_, _> = topics.iter().map(|(k, v)| (k, v.receiver_count())).collect();
        let queue_stats: HashMap<_, _> = queues.iter().map(|(k, v)| (k, v.len())).collect();
        
        serde_json::json!({
            "topics": topic_stats,
            "queues": queue_stats
        })
    }
}
