# Bus9

**Bus9** is a minimalist, ultra-lightweight "all-in-one" message broker. It combines the high performance of Rust with a beautiful React-based dashboard. Designed for those who are tired of complex Kafka or RabbitMQ configurations for simple tasks.

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [Installation and Setup](#-installation-and-setup)
- [Configuration](#️-configuration)
- [Key Features](#-key-features)
- [API Documentation](#-api-documentation)
- [Usage Examples](#-usage-examples)
- [Advanced Features](#-advanced-features)
- [Web Interface](#️-web-interface)
- [Benchmarking](#-benchmarking)
- [Technology Stack](#️-technology-stack)

---

## 🚀 Quick Start

### The Easiest Way (Docker)

```bash
docker run -p 8080:8080 denyzhirkov/bus9
```

Open in your browser: `http://localhost:8080`

### Running from Source

```bash
# Clone the repository
git clone https://github.com/denyzhirkov/bus9.git
cd bus9

# Use the dev script (builds frontend and runs backend)
./run_local.sh
```

---

## 📦 Installation and Setup

### Requirements

- **Rust** (for building from source)
- **Node.js** (for building frontend)
- **Docker** (optional, for containerized deployment)

### Method 1: Docker (Recommended for Production)

#### Basic Run
```bash
docker run -d \
  --name bus9 \
  -p 8080:8080 \
  denyzhirkov/bus9
```

#### With Custom Parameters
```bash
docker run -d \
  --name bus9 \
  -p 8080:8080 \
  -e BUS9_PORT=9090 \
  -e BUS9_AUTH_TOKEN=my-secret-token \
  -e BUS9_TOPIC_REPLAY_COUNT=10 \
  denyzhirkov/bus9
```

#### With Authentication
```bash
docker run -d \
  --name bus9 \
  -p 8080:8080 \
  -e BUS9_AUTH_TOKEN=your-secret-token-here \
  denyzhirkov/bus9
```

### Method 2: From Source

#### Step 1: Clone Repository
```bash
git clone https://github.com/denyzhirkov/bus9.git
cd bus9
```

#### Step 2: Build Frontend
```bash
cd front
npm install
npm run build
cd ..
```

#### Step 3: Build and Run Backend
```bash
# Development mode
cargo run

# Production mode (optimized build)
cargo build --release
./target/release/bus9
```

#### Using Dev Script
```bash
# Automatically builds frontend and runs backend
./run_local.sh
```

### Method 3: Using Pre-built Binary

After building, the binary is located at `target/release/bus9`. You can run it directly:

```bash
./target/release/bus9 --port 8080
```

---

## ⚙️ Configuration

Bus9 follows a **zero-config** philosophy — it works out of the box without any configuration. All parameters are optional and can be set via command-line arguments or environment variables.

### Configuration Parameters

| Parameter | Environment Variable | Default | Description |
|-----------|---------------------|---------|-------------|
| `--port` | `BUS9_PORT` | `8080` | Port to listen on |
| `--host` | `BUS9_HOST` | `0.0.0.0` | Host to bind to (0.0.0.0 = all interfaces) |
| `--topic-capacity` | `BUS9_TOPIC_CAPACITY` | `1024` | Broadcast channel capacity for topics |
| `--max-expired` | `BUS9_MAX_EXPIRED` | `50` | Maximum number of expired events to keep in history |
| `--sweep-interval-ms` | `BUS9_SWEEP_INTERVAL_MS` | `1000` | Expired resources cleanup interval (ms) |
| `--stats-interval-ms` | `BUS9_STATS_INTERVAL_MS` | `500` | Stats WebSocket update interval (ms) |
| `--topic-replay-count` | `BUS9_TOPIC_REPLAY_COUNT` | `0` | Number of messages to replay to new subscribers (0 = disabled) |
| `--inactivity-timeout-seconds` | `BUS9_INACTIVITY_TIMEOUT_SECONDS` | `None` | Inactivity timeout for auto-cleanup (seconds, disabled by default) |
| `--topic-exclude-patterns` | `BUS9_TOPIC_EXCLUDE_PATTERNS` | `[]` | Comma-separated list of topic patterns to exclude from wildcard subscriptions |
| `--auth-token` | `BUS9_AUTH_TOKEN` | `None` | Global authentication token. If set, all API requests must include this token as query parameter `?token=<token>` |

### Configuration Examples

#### Minimal Configuration (Zero-Config)
```bash
./bus9
# Runs on port 8080, no authentication, all features with defaults
```

#### Production Configuration with Authentication
```bash
./bus9 \
  --port 9090 \
  --host 0.0.0.0 \
  --auth-token "production-secret-token-2024" \
  --topic-replay-count 50 \
  --inactivity-timeout-seconds 3600 \
  --topic-exclude-patterns "internal.secret,admin.**"
```

#### Using Environment Variables
```bash
export BUS9_PORT=9090
export BUS9_AUTH_TOKEN=my-secret-token
export BUS9_TOPIC_REPLAY_COUNT=20
./bus9
```

#### Docker with Environment Variables
```bash
docker run -d \
  -p 9090:9090 \
  -e BUS9_PORT=9090 \
  -e BUS9_AUTH_TOKEN=my-secret-token \
  -e BUS9_TOPIC_REPLAY_COUNT=20 \
  -e BUS9_INACTIVITY_TIMEOUT_SECONDS=1800 \
  denyzhirkov/bus9
```

---

## ✨ Key Features

### 1. Pub/Sub (Publish/Subscribe)
- **Message Broadcasting** — one message delivered to all subscribers
- **WebSocket Support** — native support for browsers
- **HTTP API** — simple REST API for any programming language
- **Use Cases**: Live updates, chat applications, notifications, monitoring

### 2. Queues (Task Queues)
- **Point-to-Point** — each message delivered to exactly one consumer
- **Persistence** — messages stored in memory until processed
- **Priorities** — support for high/normal/low priorities
- **Use Cases**: Task distribution, load balancing, background jobs

### 3. TTL (Time-To-Live)
- **TTL for Topics/Queues** — automatic removal after inactivity period
- **TTL for Messages** — optional message expiration
- **Flexible Configuration** — can be combined with auto-cleanup

### 4. Message Replay
- **Message History** — new subscribers receive last N messages
- **Configurable Size** — number of messages for replay
- **Use Cases**: Reconnection recovery, context initialization

### 5. Pattern-based Subscription
- **Wildcard Patterns** — subscribe to multiple topics simultaneously
- **Syntax**: `*` (single segment), `**` (any path)
- **Auto-connection** — new topics automatically added to subscription
- **Exclusions** — ability to exclude specific topics from patterns

### 6. Queue Priority
- **Three Priority Levels**: high, normal, low
- **Priority Processing** — high always processed first
- **Backward Compatible** — no priority specified = normal

### 7. Auto-cleanup
- **Automatic Cleanup** — removes inactive topics/queues
- **Configurable Timeout** — inactivity period for removal
- **Independent of TTL** — works in parallel with TTL mechanism

### 8. Authentication
- **Simple Authentication** — token via query parameter
- **Optional** — works without authentication by default
- **API Protection** — all operations require token (if set)

---

## 📖 API Documentation

The server listens on port `8080` by default.

### Basic Endpoints

#### Health Check
```
GET /health
```
Returns `OK` if the server is running.

**Example:**
```bash
curl http://localhost:8080/health
```

#### Statistics
```
GET /api/stats
```
Returns a snapshot of broker state (topics, queues, expired).

**Example:**
```bash
curl http://localhost:8080/api/stats
```

**Response:**
```json
{
  "topics": {
    "news": {
      "subscribers": 3,
      "meta": {
        "ttl_seconds": null,
        "created_at": 1678901234567,
        "last_activity": 1678901234567
      },
      "expires_at": null
    }
  },
  "queues": {
    "jobs": {
      "depth": 5,
      "meta": {...},
      "expires_at": null
    }
  },
  "expired": [...]
}
```

#### Metrics
```
GET /api/metrics
```
Detailed metrics including request counts by type.

**Example:**
```bash
curl http://localhost:8080/api/metrics
```

#### Live Stats (WebSocket)
```
WS /api/ws/stats
```
Real-time statistics stream for the dashboard.

---

### 1. Publish Messages (Pub/Sub)

Sends a message to all active subscribers of a topic.

**Endpoint:** `POST /api/pub?topic=<TOPIC_NAME>&ttl_seconds=<OPTIONAL_SECONDS>`

**Parameters:**
- `topic` (required) — topic name
- `ttl_seconds` (optional) — TTL for topic in seconds
- `token` (optional) — authentication token, if `--auth-token` is set

**Body:** Message text or JSON

**Examples:**

```bash
# Simple publish
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news"

# With TTL
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news&ttl_seconds=60"

# With authentication
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news&token=my-secret-token"

# JSON payload
curl -X POST -H "Content-Type: application/json" \
  -d '{"message": "Hello", "user": "alice"}' \
  "http://localhost:8080/api/pub?topic=chat"

# JSON with TTL in body
curl -X POST -H "Content-Type: application/json" \
  -d '{"payload": "Hello World", "ttl_seconds": 120}' \
  "http://localhost:8080/api/pub?topic=news"
```

**Response:**
```json
{
  "count": 3,
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

---

### 2. Subscribe to Messages (WebSocket)

Connects to a topic and receives messages in real-time.

**URL:** `ws://localhost:8080/api/sub?topic=<TOPIC_NAME>&ttl_seconds=<OPTIONAL_SECONDS>`

**Parameters:**
- `topic` (required) — topic name or pattern
- `ttl_seconds` (optional) — TTL for subscription
- `token` (optional) — authentication token

**Examples:**

#### JavaScript (Browser)
```javascript
// Simple subscription
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=news');

ws.onopen = () => console.log('Connected');
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Received:', msg.payload);
};
ws.onerror = (error) => console.error('Error:', error);
ws.onclose = () => console.log('Disconnected');
```

#### JavaScript (with Authentication)
```javascript
const token = 'my-secret-token';
const ws = new WebSocket(`ws://localhost:8080/api/sub?topic=news&token=${token}`);

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Message:', msg);
};
```

#### Node.js
```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8080/api/sub?topic=news');

ws.on('message', (data) => {
  const msg = JSON.parse(data);
  console.log('Received:', msg);
});
```

#### Python
```python
import asyncio
import websockets
import json

async def subscribe():
    uri = "ws://localhost:8080/api/sub?topic=news"
    async with websockets.connect(uri) as websocket:
        async for message in websocket:
            msg = json.loads(message)
            print(f"Received: {msg['payload']}")

asyncio.run(subscribe())
```

#### Python (with Authentication)
```python
uri = "ws://localhost:8080/api/sub?topic=news&token=my-secret-token"
```

#### Go
```go
package main

import (
    "encoding/json"
    "log"
    "github.com/gorilla/websocket"
)

func main() {
    url := "ws://localhost:8080/api/sub?topic=news"
    c, _, err := websocket.DefaultDialer.Dial(url, nil)
    if err != nil {
        log.Fatal("dial:", err)
    }
    defer c.Close()

    for {
        _, message, err := c.ReadMessage()
        if err != nil {
            log.Println("read:", err)
            return
        }
        var msg map[string]interface{}
        json.Unmarshal(message, &msg)
        log.Printf("Received: %v", msg)
    }
}
```

---

### 3. Pattern-based Subscription (Wildcard Subscriptions)

Subscribe to multiple topics simultaneously using patterns.

**Pattern Syntax:**
- `*` — matches a single segment (e.g., `sensor.*` matches `sensor.temp` but not `sensor.room.temp`)
- `**` — matches any path (e.g., `logs.**` matches `logs.app.error`, `logs.system.info`)

**Examples:**

```javascript
// Subscribe to all sensors
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=sensor.*');

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(`Sensor ${msg.topic}: ${msg.payload}`);
};
```

```javascript
// Subscribe to all logs
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=logs.**');
```

**Important:** New topics matching the pattern are automatically added to the subscription.

#### Topic Exclusions

You can exclude specific topics from wildcard subscriptions:

```bash
# Run with exclusions
./bus9 --topic-exclude-patterns "sensor.secret,logs.admin.**,internal.*"
```

**Behavior:**
- Subscribing to `sensor.*` includes `sensor.temp` but NOT `sensor.secret`
- Subscribing to `logs.**` includes `logs.app.error` but NOT `logs.admin.access`
- Direct subscription to `sensor.secret` still works: `ws://.../api/sub?topic=sensor.secret`

---

### 4. Task Queues

Unlike Pub/Sub, messages in queues are persistent (stored in memory) until processed. Each message is delivered to **exactly one** consumer.

#### Push to Queue (Producer)

**Endpoint:** `POST /api/queue/<QUEUE_NAME>?ttl_seconds=<OPTIONAL_SECONDS>&priority=<OPTIONAL_PRIORITY>`

**Parameters:**
- `QUEUE_NAME` — queue name (in URL path)
- `ttl_seconds` (optional) — TTL for queue
- `priority` (optional) — priority: `high`, `normal`, `low` (default `normal`)
- `token` (optional) — authentication token

**Examples:**

```bash
# Basic push
curl -X POST -d "Process Image #123" "http://localhost:8080/api/queue/jobs"

# With priority
curl -X POST -d "Urgent Task" "http://localhost:8080/api/queue/jobs?priority=high"

# With TTL and priority
curl -X POST -d "Task" "http://localhost:8080/api/queue/jobs?ttl_seconds=300&priority=high"

# JSON with priority in body
curl -X POST -H "Content-Type: application/json" \
  -d '{"payload": "Process Image #123", "priority": "high"}' \
  "http://localhost:8080/api/queue/jobs"

# With authentication
curl -X POST -d "Task" \
  "http://localhost:8080/api/queue/jobs?priority=high&token=my-secret-token"
```

**Response:**
```json
{
  "status": "ok",
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Pop from Queue (Consumer/Worker)

**Endpoint:** `GET /api/queue/<QUEUE_NAME>?token=<OPTIONAL_TOKEN>`

**Examples:**

```bash
# Basic pop
curl "http://localhost:8080/api/queue/jobs"

# With authentication
curl "http://localhost:8080/api/queue/jobs?token=my-secret-token"
```

**Response (if queue is not empty):**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": "Process Image #123",
  "timestamp": 1678901234567,
  "priority": "high"
}
```

**Response (if queue is empty):**
```
HTTP 204 No Content
```

#### Priority Processing

Messages are processed in priority order: `high` → `normal` → `low`

**Example:**
```bash
# Add messages with different priorities
curl -X POST -d "Low priority" "http://localhost:8080/api/queue/jobs?priority=low"
curl -X POST -d "Normal priority" "http://localhost:8080/api/queue/jobs?priority=normal"
curl -X POST -d "High priority" "http://localhost:8080/api/queue/jobs?priority=high"

# When popping, we'll get high first, then normal, then low
curl "http://localhost:8080/api/queue/jobs"  # High priority
curl "http://localhost:8080/api/queue/jobs"  # Normal priority
curl "http://localhost:8080/api/queue/jobs"  # Low priority
```

#### Examples in Different Languages

**Python (Producer):**
```python
import requests

def add_job(payload, priority='normal', token=None):
    url = "http://localhost:8080/api/queue/jobs"
    params = {"priority": priority}
    if token:
        params["token"] = token
    
    response = requests.post(url, data=payload, params=params)
    return response.json()

# Usage
add_job("Process image", priority="high", token="my-secret-token")
```

**Python (Consumer):**
```python
import requests
import time

def process_jobs(token=None):
    url = "http://localhost:8080/api/queue/jobs"
    params = {}
    if token:
        params["token"] = token
    
    while True:
        response = requests.get(url, params=params)
        if response.status_code == 200:
            job = response.json()
            print(f"Processing: {job['payload']}")
            # Process the job
        elif response.status_code == 204:
            print("Queue is empty, waiting...")
            time.sleep(1)
        else:
            print(f"Error: {response.status_code}")
            break

process_jobs(token="my-secret-token")
```

**Go (Producer):**
```go
package main

import (
    "bytes"
    "net/http"
    "net/url"
)

func addJob(payload string, priority string, token string) error {
    baseURL := "http://localhost:8080/api/queue/jobs"
    u, _ := url.Parse(baseURL)
    q := u.Query()
    q.Set("priority", priority)
    if token != "" {
        q.Set("token", token)
    }
    u.RawQuery = q.Encode()

    resp, err := http.Post(u.String(), "text/plain", bytes.NewBufferString(payload))
    if err != nil {
        return err
    }
    defer resp.Body.Close()
    return nil
}
```

**Go (Consumer):**
```go
func processJobs(token string) {
    for {
        url := "http://localhost:8080/api/queue/jobs"
        if token != "" {
            url += "?token=" + token
        }
        
        resp, err := http.Get(url)
        if err != nil {
            log.Println("Error:", err)
            continue
        }
        
        if resp.StatusCode == 200 {
            var job map[string]interface{}
            json.NewDecoder(resp.Body).Decode(&job)
            log.Printf("Processing: %v", job["payload"])
        } else if resp.StatusCode == 204 {
            log.Println("Queue empty, waiting...")
            time.Sleep(1 * time.Second)
        }
        resp.Body.Close()
    }
}
```

---

## 💡 Usage Examples

### Scenario 1: Live Updates in Web Application

**Task:** Send order status updates to all connected users.

**Solution:**

```javascript
// Backend: publish update
fetch('http://localhost:8080/api/pub?topic=orders', {
  method: 'POST',
  body: JSON.stringify({
    orderId: '12345',
    status: 'shipped',
    timestamp: Date.now()
  })
});

// Frontend: subscribe to updates
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=orders');
ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  updateOrderStatus(update.orderId, update.status);
};
```

### Scenario 2: Task Distribution Among Workers

**Task:** Process images with multiple workers.

**Solution:**

```python
# Producer: adds tasks to queue
import requests

def add_image_task(image_id):
    requests.post(
        'http://localhost:8080/api/queue/image-processing',
        data=f"Process image {image_id}",
        params={"priority": "normal"}
    )

# Worker: processes tasks
def worker():
    while True:
        response = requests.get('http://localhost:8080/api/queue/image-processing')
        if response.status_code == 200:
            task = response.json()
            process_image(task['payload'])
        else:
            time.sleep(1)
```

### Scenario 3: Monitoring Multiple Sensors

**Task:** Subscribe to all sensors simultaneously.

**Solution:**

```javascript
// Subscribe to all sensors via pattern
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=sensor.*');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  const sensorName = extractSensorName(data); // extract from payload
  updateSensorDisplay(sensorName, data);
};
```

### Scenario 4: Priority Task Processing

**Task:** Critical tasks must be processed first.

**Solution:**

```bash
# Add critical task
curl -X POST -d "Critical system update" \
  "http://localhost:8080/api/queue/tasks?priority=high"

# Add regular task
curl -X POST -d "Regular maintenance" \
  "http://localhost:8080/api/queue/tasks?priority=normal"

# Worker will always get critical task first
```

### Scenario 5: Protected Broker with Authentication

**Task:** Protect broker from unauthorized access.

**Solution:**

```bash
# Run with token
./bus9 --auth-token "production-secret-2024"

# All requests now require token
curl -X POST -d "Hello" \
  "http://localhost:8080/api/pub?topic=news&token=production-secret-2024"
```

```javascript
// Frontend with token
const token = 'production-secret-2024';
const ws = new WebSocket(
  `ws://localhost:8080/api/sub?topic=news&token=${token}`
);
```

---

## 🔧 Advanced Features

### Message Replay

When enabled via `--topic-replay-count`, new subscribers automatically receive the last N messages before receiving live updates.

**Use Cases:**
- Reconnection recovery
- Initializing new subscribers with context
- Debugging and monitoring

**Example:**
```bash
# Enable replay of last 10 messages
./bus9 --topic-replay-count 10
```

**How it Works:**
1. Server saves last N messages for each topic
2. When a new subscriber connects, all saved messages are sent first
3. Then subscriber receives live updates

**Code Example:**
```javascript
// Connect to topic with replay
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=news');

let isReplay = true;
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (isReplay) {
    console.log('Replay message:', msg);
    // First N messages are replay
  } else {
    console.log('Live message:', msg);
  }
  // After receiving all replay messages, switch to live
  isReplay = false;
};
```

### Auto-cleanup

Automatically removes topics and queues that have been inactive for a specified period.

**Use Cases:**
- Preventing accumulation of "dead" resources
- Memory savings
- Automatic cleanup of temporary topics

**Example:**
```bash
# Remove topics/queues inactive for more than 5 minutes
./bus9 --inactivity-timeout-seconds 300
```

**How it Works:**
- Server tracks `last_activity` for each topic/queue
- Periodically (every second by default) checks inactive resources
- Removes resources inactive longer than the specified timeout

**Combination with TTL:**
Auto-cleanup works independently of TTL. A resource will be removed if either condition is met:
- TTL expired
- Inactivity period exceeded timeout

**Example:**
```bash
# Combination of TTL and auto-cleanup
./bus9 --inactivity-timeout-seconds 600

# Topic will be removed if:
# - TTL expired (if set), OR
# - Inactive for more than 10 minutes
```

### Authentication

Simple token-based authentication for protecting the broker.

**When to Use:**
- Production environment
- Protection from unauthorized access
- Multi-user scenarios

**Example:**
```bash
# Run with authentication
./bus9 --auth-token "my-secret-token-123"
```

**Usage in Code:**

**HTTP Requests:**
```bash
curl -X POST -d "Hello" \
  "http://localhost:8080/api/pub?topic=news&token=my-secret-token-123"
```

**WebSocket:**
```javascript
const token = 'my-secret-token-123';
const ws = new WebSocket(
  `ws://localhost:8080/api/sub?topic=news&token=${token}`
);
```

**Python:**
```python
import requests

url = "http://localhost:8080/api/pub"
params = {
    "topic": "news",
    "token": "my-secret-token-123"
}
requests.post(url, data="Hello", params=params)
```

**Important:**
- If `--auth-token` is not set, server works without authentication
- Endpoints `/health`, `/api/stats`, `/api/metrics` are not protected by authentication
- Token is passed only via query parameter `?token=<token>`

---

## 🖥️ Web Interface (Dashboard)

Bus9 includes a built-in web dashboard for monitoring and management.

### Accessing the Dashboard

After starting the server, open in your browser:
```
http://localhost:8080
```

### Dashboard Features

1. **Live Metrics** — real-time metrics:
   - Total request count
   - Active topics and queues
   - Queue depths
   - Subscriber counts

2. **Active Topics** — list of active topics:
   - Subscriber count
   - TTL information
   - Creation time and last activity

3. **Active Queues** — list of active queues:
   - Queue depth (message count)
   - TTL information
   - Queue status

4. **Recently Expired** — history of expired resources:
   - Removal reason (TTL or inactivity)
   - Expiration time
   - Resource type (topic/queue)

5. **Publish Message** — manual message publishing:
   - Mode selection (Topic or Queue)
   - Target topic/queue specification
   - TTL configuration
   - Test message sending

6. **Live Monitor** — real-time message monitoring:
   - Topic selection for monitoring
   - Real-time message stream
   - History of last 50 messages

### Using the Dashboard

#### Publishing a Test Message

1. In the "Publish Message" section, select mode (Topic or Queue)
2. Enter topic/queue name
3. Enter message text
4. (Optional) Specify TTL in seconds
5. Click "Send"

#### Monitoring a Topic

1. In the "Live Monitor" section, enter topic name
2. Watch messages in real-time
3. Messages are displayed with timestamp and ID

---

## 🧪 Benchmarking

Bus9 includes a built-in tool for load testing.

### Running the Benchmark

```bash
./run_bench.sh
```

This script runs a Rust-based benchmark tool (`tests/bench`) that:
- Tests publish performance
- Tests subscription performance
- Tests queue performance
- Measures latency and throughput

### Benchmark Results

The benchmark outputs statistics:
- Operations per second
- Average latency
- Latency percentiles (p50, p95, p99)
- Memory usage

---

## 🛠 Technology Stack

- **Backend**: Rust (Axum, Tokio) — for maximum speed and reliability
- **Frontend**: React (Vite, Framer Motion) — for smooth and responsive UI
- **Transport**: HTTP/1.1 & WebSocket
- **Embedding**: rust-embed — frontend embedded in binary

---

## 📝 Message Format

All messages in Bus9 have the following format:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": "Message content",
  "timestamp": 1678901234567,
  "priority": "normal"
}
```

**Fields:**
- `id` — unique message identifier (UUID v4)
- `payload` — message content (string)
- `timestamp` — timestamp in milliseconds (Unix timestamp)
- `priority` — priority (for queues only): `high`, `normal`, `low` or `null`

---

## 🔒 Security

### Production Recommendations

1. **Use Authentication:**
   ```bash
   ./bus9 --auth-token "strong-random-token-here"
   ```

2. **Restrict Network Access:**
   - Use firewall to restrict access
   - Run on internal network, not on public interface

3. **Use HTTPS/WSS:**
   - Deploy Bus9 behind reverse proxy (nginx, Caddy) with SSL
   - Configure WSS for WebSocket connections

4. **Rotate Tokens Regularly:**
   - Periodically change `--auth-token`
   - Use strong random tokens

5. **Monitoring:**
   - Monitor metrics via `/api/metrics`
   - Set up alerts for unusual activity

---

## 🐛 Troubleshooting

### Issue: Server Won't Start

**Solution:**
```bash
# Check if port is in use
lsof -i :8080

# Use a different port
./bus9 --port 9090
```

### Issue: WebSocket Won't Connect

**Solution:**
- Check that server is running
- Ensure you're using correct protocol (`ws://` for HTTP, `wss://` for HTTPS)
- Check authentication (if token is used)

### Issue: Messages Not Delivered

**Solution:**
- Check that there are active subscribers
- Ensure topic hasn't expired (TTL)
- Check server logs

### Issue: High Memory Usage

**Solution:**
- Enable auto-cleanup: `--inactivity-timeout-seconds 3600`
- Reduce replay count: `--topic-replay-count 10`
- Limit channel capacity: `--topic-capacity 512`

---

## 📄 License

MIT

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

## 📧 Support

For issues and questions, please open an issue on GitHub.
