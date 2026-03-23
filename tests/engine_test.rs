use bus9::engine::{Engine, EngineConfig, Message, MessagePriority, matches_pattern};

fn default_config() -> EngineConfig {
    EngineConfig::default()
}

fn config_with_ack(ack_timeout_ms: u64, max_retries: u32) -> EngineConfig {
    EngineConfig {
        ack_timeout_ms,
        max_retries,
        dlq_suffix: ".dlq".to_string(),
        ..EngineConfig::default()
    }
}

// ── Pub/Sub ──

#[test]
fn publish_and_subscribe() {
    let engine = Engine::new(default_config());
    let (mut rx, _) = engine.subscribe("test.topic", None).unwrap();
    let msg = Message::new("hello".into());
    let count = engine.publish("test.topic", msg, None).unwrap();
    assert_eq!(count, 1);

    let received = rx.try_recv().unwrap();
    assert_eq!(received.payload, "hello");
}

#[test]
fn publish_to_nonexistent_topic_creates_it() {
    let engine = Engine::new(default_config());
    let msg = Message::new("payload".into());
    // broadcast::channel creates an internal receiver, so count is 1 on first send
    let count = engine.publish("new.topic", msg, None).unwrap();
    assert!(count <= 1);
    // Topic should exist now
    let stats = engine.get_stats();
    assert!(stats.topics.contains_key("new.topic"));
}

#[test]
fn multiple_subscribers_receive_message() {
    let engine = Engine::new(default_config());
    let (mut rx1, _) = engine.subscribe("t", None).unwrap();
    let (mut rx2, _) = engine.subscribe("t", None).unwrap();

    engine.publish("t", Message::new("data".into()), None).unwrap();

    assert_eq!(rx1.try_recv().unwrap().payload, "data");
    assert_eq!(rx2.try_recv().unwrap().payload, "data");
}

#[test]
fn replay_history() {
    let config = EngineConfig {
        topic_replay_count: 3,
        ..default_config()
    };
    let engine = Engine::new(config);

    for i in 0..5 {
        engine.publish("t", Message::new(format!("msg{}", i)), None).unwrap();
    }

    let (_rx, history) = engine.subscribe("t", None).unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].payload, "msg2");
    assert_eq!(history[2].payload, "msg4");
}

// ── Queue ──

#[test]
fn queue_push_pop() {
    let engine = Engine::new(default_config());
    engine.push_queue("q1", Message::new("item1".into()), None).unwrap();
    engine.push_queue("q1", Message::new("item2".into()), None).unwrap();

    let m1 = engine.pop_queue("q1").unwrap();
    assert_eq!(m1.payload, "item1");
    let m2 = engine.pop_queue("q1").unwrap();
    assert_eq!(m2.payload, "item2");
    assert!(engine.pop_queue("q1").is_none());
}

#[test]
fn queue_priority_ordering() {
    let engine = Engine::new(default_config());
    engine.push_queue("q", Message::with_priority("low".into(), Some(MessagePriority::Low)), None).unwrap();
    engine.push_queue("q", Message::with_priority("normal".into(), Some(MessagePriority::Normal)), None).unwrap();
    engine.push_queue("q", Message::with_priority("high".into(), Some(MessagePriority::High)), None).unwrap();

    assert_eq!(engine.pop_queue("q").unwrap().payload, "high");
    assert_eq!(engine.pop_queue("q").unwrap().payload, "normal");
    assert_eq!(engine.pop_queue("q").unwrap().payload, "low");
}

#[test]
fn queue_depth_limit() {
    let config = EngineConfig {
        max_queue_depth: 2,
        ..default_config()
    };
    let engine = Engine::new(config);
    engine.push_queue("q", Message::new("a".into()), None).unwrap();
    engine.push_queue("q", Message::new("b".into()), None).unwrap();
    let err = engine.push_queue("q", Message::new("c".into()), None).unwrap_err();
    assert!(err.contains("full"));
}

#[test]
fn pop_empty_queue_returns_none() {
    let engine = Engine::new(default_config());
    assert!(engine.pop_queue("nonexistent").is_none());
}

// ── Validation ──

#[test]
fn validate_empty_name() {
    let engine = Engine::new(default_config());
    assert!(engine.validate_name("").is_err());
}

#[test]
fn validate_long_name() {
    let config = EngineConfig {
        max_name_length: 10,
        ..default_config()
    };
    let engine = Engine::new(config);
    assert!(engine.validate_name("short").is_ok());
    assert!(engine.validate_name("this_is_too_long").is_err());
}

#[test]
fn validate_payload_size() {
    let config = EngineConfig {
        max_payload_size: 5,
        ..default_config()
    };
    let engine = Engine::new(config);
    assert!(engine.validate_payload_size("ok").is_ok());
    assert!(engine.validate_payload_size("too_long_payload").is_err());
}

#[test]
fn max_topics_limit() {
    let config = EngineConfig {
        max_topics: 2,
        ..default_config()
    };
    let engine = Engine::new(config);
    engine.publish("t1", Message::new("a".into()), None).unwrap();
    engine.publish("t2", Message::new("b".into()), None).unwrap();
    let err = engine.publish("t3", Message::new("c".into()), None).unwrap_err();
    assert!(err.contains("Maximum number of topics"));
}

#[test]
fn max_queues_limit() {
    let config = EngineConfig {
        max_queues: 1,
        ..default_config()
    };
    let engine = Engine::new(config);
    engine.push_queue("q1", Message::new("a".into()), None).unwrap();
    let err = engine.push_queue("q2", Message::new("b".into()), None).unwrap_err();
    assert!(err.contains("Maximum number of queues"));
}

// ── Pattern matching ──

#[test]
fn pattern_single_wildcard() {
    assert!(matches_pattern("sensor.temp", "sensor.*"));
    assert!(matches_pattern("sensor.humidity", "sensor.*"));
    assert!(!matches_pattern("sensor.room.temp", "sensor.*"));
}

#[test]
fn pattern_double_wildcard() {
    assert!(matches_pattern("logs.app.error", "logs.**"));
    assert!(matches_pattern("logs.system.info", "logs.**"));
    // "logs" without dot doesn't match "logs.**" (prefix is "logs.")
    assert!(!matches_pattern("logs", "logs.**"));
}

#[test]
fn pattern_exact_match() {
    assert!(matches_pattern("foo.bar", "foo.bar"));
    assert!(!matches_pattern("foo.bar", "foo.baz"));
}

#[test]
fn pattern_star_matches_everything() {
    assert!(matches_pattern("anything", "**"));
    assert!(matches_pattern("a.b.c", "**"));
}

#[test]
fn pattern_subscribe() {
    let engine = Engine::new(default_config());
    engine.publish("sensor.temp", Message::new("1".into()), None).unwrap();
    engine.publish("sensor.humidity", Message::new("2".into()), None).unwrap();
    engine.publish("logs.error", Message::new("3".into()), None).unwrap();

    let subs = engine.subscribe_pattern("sensor.*", None).unwrap();
    assert_eq!(subs.len(), 2);
}

// ── ACK ──

#[test]
fn ack_disabled_by_default() {
    let engine = Engine::new(default_config());
    let err = engine.ack_queue("q", "some-id").unwrap_err();
    assert!(err.contains("disabled"));
}

#[test]
fn ack_flow() {
    let engine = Engine::new(config_with_ack(60_000, 0));
    engine.push_queue("q", Message::new("payload".into()), None).unwrap();

    let msg = engine.pop_queue("q").unwrap();
    // Queue should be empty now (message is in pending)
    assert!(engine.pop_queue("q").is_none());

    // ACK the message
    engine.ack_queue("q", &msg.id).unwrap();
    // Double ACK should fail
    assert!(engine.ack_queue("q", &msg.id).is_err());
}

#[test]
fn nack_returns_to_queue() {
    let engine = Engine::new(config_with_ack(60_000, 0));
    engine.push_queue("q", Message::new("payload".into()), None).unwrap();

    let msg = engine.pop_queue("q").unwrap();
    assert!(engine.pop_queue("q").is_none());

    // NACK returns message to queue
    engine.nack_queue("q", &msg.id).unwrap();
    let returned = engine.pop_queue("q").unwrap();
    assert_eq!(returned.payload, "payload");
}

#[test]
fn ack_wrong_queue() {
    let engine = Engine::new(config_with_ack(60_000, 0));
    engine.push_queue("q1", Message::new("p".into()), None).unwrap();
    let msg = engine.pop_queue("q1").unwrap();

    let err = engine.ack_queue("q2", &msg.id).unwrap_err();
    assert!(err.contains("does not belong"));
}

// ── Stats ──

#[test]
fn stats_snapshot() {
    let engine = Engine::new(default_config());
    engine.publish("t1", Message::new("a".into()), None).unwrap();
    engine.push_queue("q1", Message::new("b".into()), None).unwrap();

    let stats = engine.get_stats();
    assert!(stats.topics.contains_key("t1"));
    assert!(stats.queues.contains_key("q1"));
    assert_eq!(stats.queues["q1"].depth, 1);
}

// ── TTL / Sweep ──

#[test]
fn sweep_expired_topics() {
    let engine = Engine::new(default_config());
    // Create topic with 0-second TTL (immediately expired)
    engine.publish("ephemeral", Message::new("x".into()), Some(0)).unwrap();

    // Wait briefly so sweep can detect expiry
    std::thread::sleep(std::time::Duration::from_millis(10));
    engine.sweep_expired();

    let stats = engine.get_stats();
    assert!(!stats.topics.contains_key("ephemeral"));
    assert!(!stats.expired.is_empty());
}

#[test]
fn sweep_expired_queues() {
    let engine = Engine::new(default_config());
    engine.push_queue("temp_q", Message::new("x".into()), Some(0)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));
    engine.sweep_expired();

    let stats = engine.get_stats();
    assert!(!stats.queues.contains_key("temp_q"));
}

// ── Topic exclusion ──

#[test]
fn topic_exclusion_patterns() {
    let config = EngineConfig {
        topic_exclude_patterns: vec!["secret.**".to_string()],
        ..default_config()
    };
    let engine = Engine::new(config);
    engine.publish("public.data", Message::new("a".into()), None).unwrap();
    engine.publish("secret.keys", Message::new("b".into()), None).unwrap();

    let matching = engine.find_matching_topics("**");
    assert!(matching.contains(&"public.data".to_string()));
    assert!(!matching.contains(&"secret.keys".to_string()));
}
