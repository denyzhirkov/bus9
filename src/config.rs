use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct AppArgs {
    /// Port to listen on
    #[arg(long, env = "BUS9_PORT", default_value_t = 8080)]
    pub port: u16,

    /// Host to bind to
    #[arg(long, env = "BUS9_HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Initial capacity for broadcast channels
    #[arg(long, env = "BUS9_TOPIC_CAPACITY", default_value_t = 1024)]
    pub topic_capacity: usize,

    /// Maximum number of expired events to keep in history
    #[arg(long, env = "BUS9_MAX_EXPIRED", default_value_t = 50)]
    pub max_expired: usize,

    /// Cleanup interval in milliseconds
    #[arg(long, env = "BUS9_SWEEP_INTERVAL_MS", default_value_t = 1000)]
    pub sweep_interval_ms: u64,

    /// Stats update interval in milliseconds
    #[arg(long, env = "BUS9_STATS_INTERVAL_MS", default_value_t = 500)]
    pub stats_interval_ms: u64,

    /// Number of messages to replay to new subscribers (0 = disabled)
    #[arg(long, env = "BUS9_TOPIC_REPLAY_COUNT", default_value_t = 0)]
    pub topic_replay_count: usize,

    /// Inactivity timeout in seconds for auto-cleanup (0 = disabled)
    #[arg(long, env = "BUS9_INACTIVITY_TIMEOUT_SECONDS")]
    pub inactivity_timeout_seconds: Option<u64>,

    /// Topic exclusion patterns (comma-separated). Topics matching these patterns will be excluded from wildcard subscriptions.
    /// Example: --topic-exclude-patterns "sensor.secret,logs.admin.**"
    #[arg(long, env = "BUS9_TOPIC_EXCLUDE_PATTERNS", value_delimiter = ',')]
    pub topic_exclude_patterns: Vec<String>,

    /// Global authentication token. If set, all API requests (pub, sub, queue) must include this token.
    /// Token must be provided as query parameter: ?token=<token>
    #[arg(long, env = "BUS9_AUTH_TOKEN")]
    pub auth_token: Option<String>,

    /// Maximum payload size in bytes (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_PAYLOAD_SIZE", default_value_t = 10 * 1024 * 1024)]
    pub max_payload_size: usize,

    /// Maximum topic/queue name length in characters (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_NAME_LENGTH", default_value_t = 256)]
    pub max_name_length: usize,

    /// Maximum number of topics (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_TOPICS", default_value_t = 0)]
    pub max_topics: usize,

    /// Maximum number of queues (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_QUEUES", default_value_t = 0)]
    pub max_queues: usize,

    /// Maximum queue depth (0 = unlimited)
    #[arg(long, env = "BUS9_MAX_QUEUE_DEPTH", default_value_t = 0)]
    pub max_queue_depth: usize,
}

pub struct AppConfig {
    pub stats_interval_ms: u64,
}

impl AppConfig {
    pub fn new(stats_interval_ms: u64) -> Self {
        Self { stats_interval_ms }
    }
}
