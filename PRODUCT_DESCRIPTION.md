# Bus9: Minimalist Queuing Service

## Vision
Bus9 is a "Zen" approach to message queuing. It aims to be the most lightweight, simple-to-use, and deployable message broker. It strips away complex enterprise features in favor of raw simplicity, speed, and ease of use. It is designed to be deployed as a single Docker container that "just works".

## Core Philosophy
- **Radical Simplicity**: If a feature complicates the user experience, it is rejected.
- **Zero Friction**: No complex configuration files, no zookeeper dependence, no JVM.
- **Single Artifact**: The entire product (Server + UI) is a single executable/container.
- **Resource Efficiency**: Minimal RAM and CPU usage.

## Technical Architecture

### Backend (Rust)
- **Language**: Rust (Safe, Fast, Low footprint).
- **Role**: Handles connection management, message routing, persistence, and serves the frontend as static assets.
- **Design**: Asynchronous, event-driven core.

### Frontend (React)
- **Framework**: React.
- **Role**: Provides a clean, real-time dashboard for monitoring queues, streams, consumers, and publishing test messages.
- **Design**: Embedded directly into the Rust binary. No separate build server needed at runtime.

### Distribution (Docker)
- **Image**: Multi-stage build resulting in a scratch or alpine-based image.
- **Goal**: Image size < 20MB (compressed).

## Feature Set (MVP)
1.  **Patterns**:
    -   **Pub/Sub**: Broadcast messages to all subscribers.
    -   **Queue**: Distribute messages to one available worker (Load Balancing).
2.  **Transport**:
    -   Lightweight TCP or HTTP/WebSocket interface.
3.  **Persistence**:
    -   In-memory by default (fastest).
    -   Optional robust file-based persistence (Append-only logs).
4.  **Management UI**:
    -   Visual topology of producers/consumers.
    -   Message rates/counters.
    -   Dead letter queue inspection.

## Development Roadmap
1.  **Core**: Implement basic in-memory pub/sub engine in Rust.
2.  **API**: Define a simple JSON or Binary protocol.
3.  **UI**: Build a React dashboard to visualize the system state.
4.  **Integration**: Embed UI assets into the Rust binary.
5.  **Packaging**: Optimize Docker build.
