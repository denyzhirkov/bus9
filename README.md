# Bus9

Bus9 is a minimalist, ultra-lightweight "all-in-one" message broker. It combines the high performance of Rust with a beautiful React-based dashboard. Designed for those who are tired of complex Kafka or RabbitMQ configurations for simple tasks.

## 🚀 Quick Start

### The Easiest Way (Docker)
Run Bus9 with a single command:

```bash
docker run -p 8080:8080 denyzhirkov/bus9
```

Open in your browser: `http://localhost:8080`

### Running from Source
You will need Rust and Node.js.

```bash
# 1. Clone the repository
git clone https://github.com/denyzhirkov/bus9.git
cd bus9

# 2. Run (automatically builds frontend and starts server)
# Note: For the first run, you might need to build the frontend manually or use the Dockerfile
cd front && npm install && npm run build && cd ..
cargo run
```

## ✨ Key Features

-   **Zero Configuration**: No config files, XML, or YAML. Just run the binary.
-   **All-in-One**: Server and UI are delivered as a single executable (or Docker image).
-   **Pub/Sub**: Broadcast messages to multiple subscribers (Live updates, chat apps).
-   **Queues**: Task queues with delivery guarantees to a single worker (Load balancing).
-   **WebSocket First**: Out-of-the-box WebSocket support for browser clients.
-   **HTTP API**: Simple REST-like API for any programming language.

## 📖 API Documentation

The server listens on port `8080` by default.

### 1. Send Messages (Pub/Sub)
Sends a message to all active subscribers of a topic. Retained in memory only during delivery.

**Endpoint**: `POST /api/pub?topic=<TOPIC_NAME>`
**Body**: Message text or JSON.

```bash
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news"
```

### 2. Subscribe to Messages (WebSocket)
Connects to a topic and receives messages in real-time.

**URL**: `ws://localhost:8080/api/sub?topic=<TOPIC_NAME>`

Example (JavaScript):
```javascript
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=news');
ws.onmessage = (event) => console.log('Received:', event.data);
```

### 3. Task Queues (Queue)
Unlike Pub/Sub, messages in a queue are persisted (in memory) until a worker picks them up. Each message is delivered to **exactly one** consumer.

#### Push to Queue (Producer)
**Endpoint**: `POST /api/queue/<QUEUE_NAME>`

```bash
curl -X POST -d "Process Image #123" "http://localhost:8080/api/queue/jobs"
```

#### Pop from Queue (Consumer/Worker)
**Endpoint**: `GET /api/queue/<QUEUE_NAME>`
Returns the message (JSON) or `204 No Content` if the queue is empty.

```bash
curl "http://localhost:8080/api/queue/jobs"
```
**Response**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": "Process Image #123",
  "timestamp": 1678901234567
}
```

### 4. Stats
Get the current state of the broker (message counts, subscribers).

**Endpoint**: `GET /api/stats`

```bash
curl "http://localhost:8080/api/stats"
```

## 🖥️ Web Interface (Dashboard)

The built-in dashboard allows you to:
-   View the list of active topics and queues.
-   Monitor message counts in real-time.
-   Manually publish test messages.
-   Watch the message stream via "Live Monitor".

Simply navigate to `http://localhost:8080` after starting the server.

## 🛠 Technology Stack

-   **Backend**: Rust (Axum, Tokio) — for maximum speed and reliability.
-   **Frontend**: React (Vite, Framer Motion) — for a smooth and responsive UI.
-   **Transport**: HTTP/1.1 & WebSocket.

## 📄 License

MIT
