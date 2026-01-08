# Bus9

**Bus9** — минималистичный, ультра-легковесный message broker "всё-в-одном". Объединяет высокую производительность Rust с красивым React-дашбордом. Создан для тех, кто устал от сложных конфигураций Kafka или RabbitMQ для простых задач.

## 📋 Содержание

- [Быстрый старт](#-быстрый-старт)
- [Установка и запуск](#-установка-и-запуск)
- [Конфигурация](#️-конфигурация)
- [Основные возможности](#-основные-возможности)
- [API документация](#-api-документация)
- [Примеры использования](#-примеры-использования)
- [Продвинутые функции](#-продвинутые-функции)
- [Веб-интерфейс](#️-веб-интерфейс)
- [Бенчмаркинг](#-бенчмаркинг)
- [Технологический стек](#️-технологический-стек)

---

## 🚀 Быстрый старт

### Самый простой способ (Docker)

```bash
docker run -p 8080:8080 denyzhirkov/bus9
```

Откройте в браузере: `http://localhost:8080`

### Запуск из исходников

```bash
# Клонируйте репозиторий
git clone https://github.com/denyzhirkov/bus9.git
cd bus9

# Используйте скрипт для разработки (собирает фронтенд и запускает бэкенд)
./run_local.sh
```

---

## 📦 Установка и запуск

### Требования

- **Rust** (для сборки из исходников)
- **Node.js** (для сборки фронтенда)
- **Docker** (опционально, для контейнерного запуска)

### Способ 1: Docker (рекомендуется для production)

#### Базовый запуск
```bash
docker run -d \
  --name bus9 \
  -p 8080:8080 \
  denyzhirkov/bus9
```

#### С кастомными параметрами
```bash
docker run -d \
  --name bus9 \
  -p 8080:8080 \
  -e BUS9_PORT=9090 \
  -e BUS9_AUTH_TOKEN=my-secret-token \
  -e BUS9_TOPIC_REPLAY_COUNT=10 \
  denyzhirkov/bus9
```

#### С авторизацией
```bash
docker run -d \
  --name bus9 \
  -p 8080:8080 \
  -e BUS9_AUTH_TOKEN=your-secret-token-here \
  denyzhirkov/bus9
```

### Способ 2: Из исходников

#### Шаг 1: Клонирование репозитория
```bash
git clone https://github.com/denyzhirkov/bus9.git
cd bus9
```

#### Шаг 2: Сборка фронтенда
```bash
cd front
npm install
npm run build
cd ..
```

#### Шаг 3: Сборка и запуск бэкенда
```bash
# Режим разработки
cargo run

# Production режим (оптимизированная сборка)
cargo build --release
./target/release/bus9
```

#### Использование скрипта для разработки
```bash
# Автоматически собирает фронтенд и запускает бэкенд
./run_local.sh
```

### Способ 3: Использование готового бинарника

После сборки бинарник находится в `target/release/bus9`. Его можно запускать напрямую:

```bash
./target/release/bus9 --port 8080
```

---

## ⚙️ Конфигурация

Bus9 работает по принципу **zero-config** — по умолчанию не требует конфигурации. Все параметры опциональны и могут быть заданы через аргументы командной строки или переменные окружения.

### Таблица параметров

| Параметр | Переменная окружения | Значение по умолчанию | Описание |
|----------|---------------------|----------------------|----------|
| `--port` | `BUS9_PORT` | `8080` | Порт для прослушивания |
| `--host` | `BUS9_HOST` | `0.0.0.0` | Хост для привязки (0.0.0.0 = все интерфейсы) |
| `--topic-capacity` | `BUS9_TOPIC_CAPACITY` | `1024` | Емкость broadcast каналов для топиков |
| `--max-expired` | `BUS9_MAX_EXPIRED` | `50` | Максимальное количество истекших событий в истории |
| `--sweep-interval-ms` | `BUS9_SWEEP_INTERVAL_MS` | `1000` | Интервал очистки истекших ресурсов (мс) |
| `--stats-interval-ms` | `BUS9_STATS_INTERVAL_MS` | `500` | Интервал обновления статистики через WebSocket (мс) |
| `--topic-replay-count` | `BUS9_TOPIC_REPLAY_COUNT` | `0` | Количество сообщений для replay новым подписчикам (0 = отключено) |
| `--inactivity-timeout-seconds` | `BUS9_INACTIVITY_TIMEOUT_SECONDS` | `None` | Таймаут неактивности для auto-cleanup (секунды, отключено по умолчанию) |
| `--topic-exclude-patterns` | `BUS9_TOPIC_EXCLUDE_PATTERNS` | `[]` | Список паттернов топиков для исключения из wildcard подписок (через запятую) |
| `--auth-token` | `BUS9_AUTH_TOKEN` | `None` | Глобальный токен авторизации. Если задан, все API запросы должны включать токен как query параметр `?token=<token>` |

### Примеры конфигурации

#### Минимальная конфигурация (zero-config)
```bash
./bus9
# Запускается на порту 8080, без авторизации, все функции по умолчанию
```

#### Production конфигурация с авторизацией
```bash
./bus9 \
  --port 9090 \
  --host 0.0.0.0 \
  --auth-token "production-secret-token-2024" \
  --topic-replay-count 50 \
  --inactivity-timeout-seconds 3600 \
  --topic-exclude-patterns "internal.secret,admin.**"
```

#### Через переменные окружения
```bash
export BUS9_PORT=9090
export BUS9_AUTH_TOKEN=my-secret-token
export BUS9_TOPIC_REPLAY_COUNT=20
./bus9
```

#### Docker с переменными окружения
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

## ✨ Основные возможности

### 1. Pub/Sub (Publish/Subscribe)
- **Broadcast сообщений** — одно сообщение доставляется всем подписчикам
- **WebSocket поддержка** — нативная поддержка для браузеров (двусторонняя связь)
- **Server-Sent Events (SSE)** — HTTP-потоковая передача для одностороннего потока данных
- **HTTP API** — простой REST API для любого языка программирования
- **Использование**: Live обновления, чаты, уведомления, мониторинг

### 3. Queues (Очереди задач)
- **Point-to-Point** — каждое сообщение доставляется только одному потребителю
- **Персистентность** — сообщения хранятся в памяти до обработки
- **Приоритеты** — поддержка high/normal/low приоритетов
- **Использование**: Распределение задач, load balancing, фоновые задачи

### 4. TTL (Time-To-Live)
- **TTL для топиков/очередей** — автоматическое удаление после периода неактивности
- **TTL для сообщений** — опциональный срок жизни сообщений
- **Гибкая настройка** — можно комбинировать с auto-cleanup

### 5. Message Replay
- **История сообщений** — новые подписчики получают последние N сообщений
- **Настраиваемый размер** — количество сообщений для replay
- **Использование**: Восстановление после переподключения, инициализация контекста

### 6. Pattern-based Subscription
- **Wildcard паттерны** — подписка на несколько топиков одновременно
- **Синтаксис**: `*` (один сегмент), `**` (любой путь)
- **Автоматическое подключение** — новые топики автоматически добавляются к подписке
- **Исключения** — возможность исключить определенные топики из паттернов

### 7. Queue Priority
- **Три уровня приоритета**: high, normal, low
- **Приоритетная обработка** — high всегда обрабатывается первым
- **Обратная совместимость** — без указания приоритета = normal

### 8. Auto-cleanup
- **Автоматическая очистка** — удаление неактивных топиков/очередей
- **Настраиваемый таймаут** — период неактивности для удаления
- **Независимость от TTL** — работает параллельно с TTL механизмом

### 9. Authentication
- **Простая авторизация** — токен через query параметр
- **Опциональная** — работает без авторизации по умолчанию
- **Защита API** — все операции требуют токен (если задан)

---

## 📖 API Документация

Сервер слушает на порту `8080` по умолчанию.

### Базовые эндпоинты

#### Health Check
```
GET /health
```
Возвращает `OK` если сервер работает.

**Пример:**
```bash
curl http://localhost:8080/health
```

#### Статистика
```
GET /api/stats
```
Возвращает снимок состояния брокера (топики, очереди, истекшие).

**Пример:**
```bash
curl http://localhost:8080/api/stats
```

**Ответ:**
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

#### Метрики
```
GET /api/metrics
```
Детальные метрики включая количество запросов по типам.

**Пример:**
```bash
curl http://localhost:8080/api/metrics
```

#### Live Stats (WebSocket)
```
WS /api/ws/stats
```
Поток статистики в реальном времени для дашборда.

---

### 1. Публикация сообщений (Pub/Sub)

Отправляет сообщение всем активным подписчикам топика.

**Endpoint:** `POST /api/pub?topic=<TOPIC_NAME>&ttl_seconds=<OPTIONAL_SECONDS>`

**Параметры:**
- `topic` (обязательный) — имя топика
- `ttl_seconds` (опциональный) — TTL для топика в секундах
- `token` (опциональный) — токен авторизации, если задан `--auth-token`

**Body:** Текст сообщения или JSON

**Примеры:**

```bash
# Простая публикация
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news"

# С TTL
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news&ttl_seconds=60"

# С авторизацией
curl -X POST -d "Hello World" "http://localhost:8080/api/pub?topic=news&token=my-secret-token"

# JSON payload
curl -X POST -H "Content-Type: application/json" \
  -d '{"message": "Hello", "user": "alice"}' \
  "http://localhost:8080/api/pub?topic=chat"

# JSON с TTL в body
curl -X POST -H "Content-Type: application/json" \
  -d '{"payload": "Hello World", "ttl_seconds": 120}' \
  "http://localhost:8080/api/pub?topic=news"
```

**Ответ:**
```json
{
  "count": 3,
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

---

### 2. Подписка на сообщения (WebSocket)

Подключается к топику и получает сообщения в реальном времени.

**URL:** `ws://localhost:8080/api/sub?topic=<TOPIC_NAME>&ttl_seconds=<OPTIONAL_SECONDS>`

**Параметры:**
- `topic` (обязательный) — имя топика или паттерн
- `ttl_seconds` (опциональный) — TTL для подписки
- `token` (опциональный) — токен авторизации

**Примеры:**

#### JavaScript (браузер)
```javascript
// Простая подписка
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=news');

ws.onopen = () => console.log('Connected');
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Received:', msg.payload);
};
ws.onerror = (error) => console.error('Error:', error);
ws.onclose = () => console.log('Disconnected');
```

#### JavaScript (с авторизацией)
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

#### Python (с авторизацией)
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

### 3. Подписка на сообщения (Server-Sent Events / SSE)

Альтернатива WebSocket для одностороннего потока данных (сервер → клиент). SSE проще в использовании и лучше работает за прокси и балансировщиками нагрузки.

**Endpoint:** `GET /api/stream?topic=<TOPIC_NAME>&ttl_seconds=<OPTIONAL_SECONDS>`

**Параметры:**
- `topic` (обязательный) — имя топика или паттерн
- `ttl_seconds` (опциональный) — TTL для подписки
- `token` (опциональный) — токен авторизации

**Преимущества SSE:**
- Проще чем WebSocket (обычный HTTP GET запрос)
- Автоматическое переподключение браузером
- Лучшая совместимость с прокси и балансировщиками
- Меньше overhead для односторонней передачи данных

**Примеры:**

#### JavaScript (браузер)
```javascript
// Простая SSE подписка
const eventSource = new EventSource('http://localhost:8080/api/stream?topic=news');

eventSource.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Received:', msg.payload);
};

eventSource.onerror = (error) => {
  console.error('SSE Error:', error);
  // Браузер автоматически переподключится
};
```

#### JavaScript (с авторизацией)
```javascript
const token = 'my-secret-token';
const eventSource = new EventSource(
  `http://localhost:8080/api/stream?topic=news&token=${token}`
);

eventSource.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Message:', msg);
};
```

#### curl (командная строка)
```bash
# Поток сообщений из топика
curl -N "http://localhost:8080/api/stream?topic=news"

# С авторизацией
curl -N "http://localhost:8080/api/stream?topic=news&token=my-secret-token"

# Подписка по паттерну
curl -N "http://localhost:8080/api/stream?topic=sensor.*"
```

#### Python
```python
import requests
import json

def subscribe_sse(topic, token=None):
    url = f"http://localhost:8080/api/stream"
    params = {"topic": topic}
    if token:
        params["token"] = token
    
    response = requests.get(url, params=params, stream=True)
    
    for line in response.iter_lines():
        if line:
            # SSE формат: "data: <json>\n\n"
            if line.startswith(b'data: '):
                data = line[6:].decode('utf-8')
                msg = json.loads(data)
                print(f"Received: {msg['payload']}")

# Использование
subscribe_sse("news", token="my-secret-token")
```

#### Node.js
```javascript
const EventSource = require('eventsource');

const eventSource = new EventSource(
  'http://localhost:8080/api/stream?topic=news&token=my-secret-token'
);

eventSource.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Received:', msg);
};

eventSource.onerror = (error) => {
  console.error('SSE Error:', error);
};
```

#### Go
```go
package main

import (
    "bufio"
    "encoding/json"
    "fmt"
    "net/http"
    "strings"
)

func subscribeSSE(topic string, token string) {
    url := fmt.Sprintf("http://localhost:8080/api/stream?topic=%s&token=%s", topic, token)
    resp, err := http.Get(url)
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()

    scanner := bufio.NewScanner(resp.Body)
    for scanner.Scan() {
        line := scanner.Text()
        if strings.HasPrefix(line, "data: ") {
            data := line[6:]
            var msg map[string]interface{}
            json.Unmarshal([]byte(data), &msg)
            fmt.Printf("Received: %v\n", msg)
        }
    }
}
```

**Когда использовать SSE vs WebSocket:**
- **Используйте SSE** когда нужен только поток сервер → клиент (односторонний)
- **Используйте WebSocket** когда нужна двусторонняя связь
- **Используйте SSE** когда нужна лучшая совместимость с прокси/балансировщиками
- **Используйте WebSocket** когда нужна минимальная задержка и полнодуплексная связь

**Примечание:** SSE поддерживает те же функции, что и WebSocket подписки:
- Подписки по паттернам (wildcard)
- Message replay (если включено)
- Авторизация через токен

---

### 4. Pattern-based Subscription (Wildcard подписки)

Подписка на несколько топиков одновременно используя паттерны.

**Синтаксис паттернов:**
- `*` — соответствует одному сегменту (например, `sensor.*` соответствует `sensor.temp`, но не `sensor.room.temp`)
- `**` — соответствует любому пути (например, `logs.**` соответствует `logs.app.error`, `logs.system.info`)

**Примеры:**

```javascript
// Подписка на все сенсоры
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=sensor.*');

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(`Sensor ${msg.topic}: ${msg.payload}`);
};
```

```javascript
// Подписка на все логи
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=logs.**');
```

**Важно:** Новые топики, соответствующие паттерну, автоматически добавляются к подписке.

#### Исключения топиков

Можно исключить определенные топики из wildcard подписок:

```bash
# Запуск с исключениями
./bus9 --topic-exclude-patterns "sensor.secret,logs.admin.**,internal.*"
```

**Поведение:**
- Подписка на `sensor.*` включает `sensor.temp`, но НЕ включает `sensor.secret`
- Подписка на `logs.**` включает `logs.app.error`, но НЕ включает `logs.admin.access`
- Прямая подписка на `sensor.secret` все еще работает: `ws://.../api/sub?topic=sensor.secret`

---

### 5. Очереди задач (Queues)

В отличие от Pub/Sub, сообщения в очередях персистентны (хранятся в памяти) до обработки. Каждое сообщение доставляется **ровно одному** потребителю.

#### Добавление в очередь (Producer)

**Endpoint:** `POST /api/queue/<QUEUE_NAME>?ttl_seconds=<OPTIONAL_SECONDS>&priority=<OPTIONAL_PRIORITY>`

**Параметры:**
- `QUEUE_NAME` — имя очереди (в пути URL)
- `ttl_seconds` (опциональный) — TTL для очереди
- `priority` (опциональный) — приоритет: `high`, `normal`, `low` (по умолчанию `normal`)
- `token` (опциональный) — токен авторизации

**Примеры:**

```bash
# Базовое добавление
curl -X POST -d "Process Image #123" "http://localhost:8080/api/queue/jobs"

# С приоритетом
curl -X POST -d "Urgent Task" "http://localhost:8080/api/queue/jobs?priority=high"

# С TTL и приоритетом
curl -X POST -d "Task" "http://localhost:8080/api/queue/jobs?ttl_seconds=300&priority=high"

# JSON с приоритетом в body
curl -X POST -H "Content-Type: application/json" \
  -d '{"payload": "Process Image #123", "priority": "high"}' \
  "http://localhost:8080/api/queue/jobs"

# С авторизацией
curl -X POST -d "Task" \
  "http://localhost:8080/api/queue/jobs?priority=high&token=my-secret-token"
```

**Ответ:**
```json
{
  "status": "ok",
  "id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### Получение из очереди (Consumer/Worker)

**Endpoint:** `GET /api/queue/<QUEUE_NAME>?token=<OPTIONAL_TOKEN>`

**Примеры:**

```bash
# Базовое получение
curl "http://localhost:8080/api/queue/jobs"

# С авторизацией
curl "http://localhost:8080/api/queue/jobs?token=my-secret-token"
```

**Ответ (если очередь не пуста):**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": "Process Image #123",
  "timestamp": 1678901234567,
  "priority": "high"
}
```

**Ответ (если очередь пуста):**
```
HTTP 204 No Content
```

#### Приоритетная обработка

Сообщения обрабатываются в порядке приоритета: `high` → `normal` → `low`

**Пример:**
```bash
# Добавляем сообщения с разными приоритетами
curl -X POST -d "Low priority" "http://localhost:8080/api/queue/jobs?priority=low"
curl -X POST -d "Normal priority" "http://localhost:8080/api/queue/jobs?priority=normal"
curl -X POST -d "High priority" "http://localhost:8080/api/queue/jobs?priority=high"

# При получении сначала получим high, потом normal, потом low
curl "http://localhost:8080/api/queue/jobs"  # High priority
curl "http://localhost:8080/api/queue/jobs"  # Normal priority
curl "http://localhost:8080/api/queue/jobs"  # Low priority
```

#### Примеры на разных языках

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

# Использование
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
            # Обработка задачи
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

## 💡 Примеры использования

### Сценарий 1: Live обновления в веб-приложении

**Задача:** Отправлять обновления статуса заказа всем подключенным пользователям.

**Решение:**

```javascript
// Backend: публикация обновления
fetch('http://localhost:8080/api/pub?topic=orders', {
  method: 'POST',
  body: JSON.stringify({
    orderId: '12345',
    status: 'shipped',
    timestamp: Date.now()
  })
});

// Frontend: подписка на обновления
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=orders');
ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  updateOrderStatus(update.orderId, update.status);
};
```

### Сценарий 2: Распределение задач между воркерами

**Задача:** Обработка изображений несколькими воркерами.

**Решение:**

```python
# Producer: добавляет задачи в очередь
import requests

def add_image_task(image_id):
    requests.post(
        'http://localhost:8080/api/queue/image-processing',
        data=f"Process image {image_id}",
        params={"priority": "normal"}
    )

# Worker: обрабатывает задачи
def worker():
    while True:
        response = requests.get('http://localhost:8080/api/queue/image-processing')
        if response.status_code == 200:
            task = response.json()
            process_image(task['payload'])
        else:
            time.sleep(1)
```

### Сценарий 3: Мониторинг множества сенсоров

**Задача:** Подписаться на все сенсоры одновременно.

**Решение:**

```javascript
// Подписка на все сенсоры через паттерн
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=sensor.*');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  const sensorName = extractSensorName(data); // извлекаем из payload
  updateSensorDisplay(sensorName, data);
};
```

### Сценарий 4: Приоритетная обработка задач

**Задача:** Критичные задачи должны обрабатываться первыми.

**Решение:**

```bash
# Добавляем критичную задачу
curl -X POST -d "Critical system update" \
  "http://localhost:8080/api/queue/tasks?priority=high"

# Добавляем обычную задачу
curl -X POST -d "Regular maintenance" \
  "http://localhost:8080/api/queue/tasks?priority=normal"

# Воркер всегда получит критичную задачу первой
```

### Сценарий 5: Защищенный брокер с авторизацией

**Задача:** Защитить брокер от несанкционированного доступа.

**Решение:**

```bash
# Запуск с токеном
./bus9 --auth-token "production-secret-2024"

# Все запросы теперь требуют токен
curl -X POST -d "Hello" \
  "http://localhost:8080/api/pub?topic=news&token=production-secret-2024"
```

```javascript
// Frontend с токеном
const token = 'production-secret-2024';
const ws = new WebSocket(
  `ws://localhost:8080/api/sub?topic=news&token=${token}`
);
```

---

## 🔧 Продвинутые функции

### Message Replay

Когда включено через `--topic-replay-count`, новые подписчики автоматически получают последние N сообщений перед получением live обновлений.

**Использование:**
- Восстановление после переподключения
- Инициализация нового подписчика с контекстом
- Отладка и мониторинг

**Пример:**
```bash
# Включить replay последних 10 сообщений
./bus9 --topic-replay-count 10
```

**Как это работает:**
1. Сервер сохраняет последние N сообщений для каждого топика
2. При подключении нового подписчика сначала отправляются все сохраненные сообщения
3. Затем подписчик получает live обновления

**Пример кода:**
```javascript
// Подключение к топику с replay
const ws = new WebSocket('ws://localhost:8080/api/sub?topic=news');

let isReplay = true;
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (isReplay) {
    console.log('Replay message:', msg);
    // Первые N сообщений - это replay
  } else {
    console.log('Live message:', msg);
  }
  // После получения всех replay сообщений переключаемся на live
  isReplay = false;
};
```

### Auto-cleanup

Автоматически удаляет топики и очереди, которые неактивны в течение заданного периода.

**Использование:**
- Предотвращение накопления "мертвых" ресурсов
- Экономия памяти
- Автоматическая очистка временных топиков

**Пример:**
```bash
# Удалять топики/очереди неактивные более 5 минут
./bus9 --inactivity-timeout-seconds 300
```

**Как это работает:**
- Сервер отслеживает `last_activity` для каждого топика/очереди
- Периодически (каждую секунду по умолчанию) проверяет неактивные ресурсы
- Удаляет ресурсы, неактивные дольше заданного таймаута

**Комбинация с TTL:**
Auto-cleanup работает независимо от TTL. Ресурс будет удален, если выполнится любое из условий:
- TTL истек
- Период неактивности превысил таймаут

**Пример:**
```bash
# Комбинация TTL и auto-cleanup
./bus9 --inactivity-timeout-seconds 600

# Топик будет удален если:
# - TTL истек (если задан), ИЛИ
# - Неактивен более 10 минут
```

### Authentication

Простая токен-базированная авторизация для защиты брокера.

**Когда использовать:**
- Production окружение
- Защита от несанкционированного доступа
- Много-пользовательские сценарии

**Пример:**
```bash
# Запуск с авторизацией
./bus9 --auth-token "my-secret-token-123"
```

**Использование в коде:**

**HTTP запросы:**
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

**Важно:**
- Если `--auth-token` не задан, сервер работает без авторизации
- Эндпоинты `/health`, `/api/stats`, `/api/metrics` не защищены авторизацией
- Токен передается только через query параметр `?token=<token>`

---

## 🖥️ Веб-интерфейс (Dashboard)

Bus9 включает встроенный веб-дашборд для мониторинга и управления.

### Доступ к дашборду

После запуска сервера откройте в браузере:
```
http://localhost:8080
```

### Возможности дашборда

1. **Live Metrics** — метрики в реальном времени:
   - Общее количество запросов
   - Активные топики и очереди
   - Глубина очередей
   - Количество подписчиков

2. **Active Topics** — список активных топиков:
   - Количество подписчиков
   - TTL информация
   - Время создания и последней активности

3. **Active Queues** — список активных очередей:
   - Глубина очереди (количество сообщений)
   - TTL информация
   - Статус очереди

4. **Recently Expired** — история истекших ресурсов:
   - Причина удаления (TTL или inactivity)
   - Время истечения
   - Тип ресурса (topic/queue)

5. **Publish Message** — ручная публикация сообщений:
   - Выбор режима (Topic или Queue)
   - Указание целевого топика/очереди
   - Настройка TTL
   - Отправка тестовых сообщений

6. **Live Monitor** — мониторинг сообщений в реальном времени:
   - Выбор топика для мониторинга
   - Поток сообщений в реальном времени
   - История последних 50 сообщений

### Использование дашборда

#### Публикация тестового сообщения

1. В разделе "Publish Message" выберите режим (Topic или Queue)
2. Введите имя топика/очереди
3. Введите текст сообщения
4. (Опционально) Укажите TTL в секундах
5. Нажмите "Send"

#### Мониторинг топика

1. В разделе "Live Monitor" введите имя топика
2. Наблюдайте за сообщениями в реальном времени
3. Сообщения отображаются с временной меткой и ID

---

## 🧪 Бенчмаркинг

Bus9 включает встроенный инструмент для нагрузочного тестирования.

### Запуск бенчмарка

```bash
./run_bench.sh
```

Этот скрипт запускает Rust-based бенчмарк инструмент (`tests/bench`), который:
- Тестирует производительность публикации
- Тестирует производительность подписки
- Тестирует производительность очередей
- Измеряет задержки и пропускную способность

### Результаты бенчмарка

Бенчмарк выводит статистику:
- Количество операций в секунду
- Средняя задержка
- Процентили задержек (p50, p95, p99)
- Использование памяти

---

## 🛠 Технологический стек

- **Backend**: Rust (Axum, Tokio) — для максимальной скорости и надежности
- **Frontend**: React (Vite, Framer Motion) — для плавного и отзывчивого UI
- **Transport**: HTTP/1.1 & WebSocket
- **Embedding**: rust-embed — фронтенд встроен в бинарник

---

## 📝 Формат сообщений

Все сообщения в Bus9 имеют следующий формат:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "payload": "Message content",
  "timestamp": 1678901234567,
  "priority": "normal"
}
```

**Поля:**
- `id` — уникальный идентификатор сообщения (UUID v4)
- `payload` — содержимое сообщения (строка)
- `timestamp` — временная метка в миллисекундах (Unix timestamp)
- `priority` — приоритет (только для очередей): `high`, `normal`, `low` или `null`

---

## 🔒 Безопасность

### Рекомендации для production

1. **Используйте авторизацию:**
   ```bash
   ./bus9 --auth-token "strong-random-token-here"
   ```

2. **Ограничьте доступ к сети:**
   - Используйте firewall для ограничения доступа
   - Запускайте на внутренней сети, не на публичном интерфейсе

3. **Используйте HTTPS/WSS:**
   - Разверните Bus9 за reverse proxy (nginx, Caddy) с SSL
   - Настройте WSS для WebSocket соединений

4. **Регулярно ротируйте токены:**
   - Периодически меняйте `--auth-token`
   - Используйте сильные случайные токены

5. **Мониторинг:**
   - Следите за метриками через `/api/metrics`
   - Настройте алерты на необычную активность

---

## 🐛 Troubleshooting

### Проблема: Сервер не запускается

**Решение:**
```bash
# Проверьте, не занят ли порт
lsof -i :8080

# Используйте другой порт
./bus9 --port 9090
```

### Проблема: WebSocket не подключается

**Решение:**
- Проверьте, что сервер запущен
- Убедитесь, что используете правильный протокол (`ws://` для HTTP, `wss://` для HTTPS)
- Проверьте авторизацию (если используется токен)

### Проблема: Сообщения не доставляются

**Решение:**
- Проверьте, что есть активные подписчики
- Убедитесь, что топик не истек (TTL)
- Проверьте логи сервера

### Проблема: Высокое использование памяти

**Решение:**
- Включите auto-cleanup: `--inactivity-timeout-seconds 3600`
- Уменьшите replay count: `--topic-replay-count 10`
- Ограничьте емкость каналов: `--topic-capacity 512`

---

## 📄 License

MIT

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

## 📧 Support

For issues and questions, please open an issue on GitHub.
