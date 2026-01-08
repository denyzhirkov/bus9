import { useEffect, useState, useRef } from 'react'
import { Activity, Radio, Server, Boxes, Send, TrendingUp } from 'lucide-react'
import './index.css'

interface EntryMeta {
  ttl_seconds: number | null
  created_at: number
  last_activity: number
}

interface TopicStatus {
  subscribers: number
  meta: EntryMeta
  expires_at: number | null
}

interface QueueStatus {
  depth: number
  meta: EntryMeta
  expires_at: number | null
}

interface ExpiredEntry {
  name: string
  kind: string
  expired_at: number
}

interface Stats {
  topics: Record<string, TopicStatus>
  queues: Record<string, QueueStatus>
  expired: ExpiredEntry[]
}

interface MetricsResponse {
  requests: Record<string, number>
  topics: {
    active: number
    entries: Record<string, TopicStatus>
  }
  queues: {
    active: number
    entries: Record<string, QueueStatus>
  }
  expired: ExpiredEntry[]
}

interface LogMessage {
  id: string
  payload: string
  timestamp: number
}

function App() {
  const [stats, setStats] = useState<Stats>({ topics: {}, queues: {}, expired: [] })
  const [metrics, setMetrics] = useState<MetricsResponse>({
    requests: {},
    topics: { active: 0, entries: {} },
    queues: { active: 0, entries: {} },
    expired: [],
  })
  const [version, setVersion] = useState<string>('')
  const [msgInput, setMsgInput] = useState('')
  const [target, setTarget] = useState('')
  const [mode, setMode] = useState<'pub' | 'queue'>('pub')
  const [logs, setLogs] = useState<LogMessage[]>([])
  const [subTopic, setSubTopic] = useState('test')
  const [ttlSeconds, setTtlSeconds] = useState('')
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
    const host = window.location.host
    const ws = new WebSocket(`${proto}://${host}/api/ws/stats`)

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        setStats(data)
      } catch (e) { console.error(e) }
    }

    return () => ws.close()
  }, [])

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const response = await fetch('/api/version')
        if (response.ok) {
          const data = await response.json()
          setVersion(data.version)
        }
      } catch (error) {
        console.error('Failed to fetch version:', error)
      }
    }

    fetchVersion()
  }, [])

  useEffect(() => {
    let active = true

    const fetchMetrics = async () => {
      try {
        const response = await fetch('/api/metrics')
        if (!response.ok) return
        const data: MetricsResponse = await response.json()
        if (!active) return
        setMetrics(data)
      } catch (error) {
        console.error(error)
      }
    }

    fetchMetrics()
    const intervalId = window.setInterval(fetchMetrics, 2000)
    return () => {
      active = false
      window.clearInterval(intervalId)
    }
  }, [])

  useEffect(() => {
    if (wsRef.current) wsRef.current.close()

    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
    const host = window.location.host
    const ws = new WebSocket(`${proto}://${host}/api/sub?topic=${subTopic}`)

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data)
        setLogs(prev => [msg, ...prev].slice(0, 50))
      } catch (e) { console.error(e) }
    }

    wsRef.current = ws
    return () => ws.close()
  }, [subTopic])

  const handleSend = async () => {
    if (!target || !msgInput) return

    const url = mode === 'pub'
      ? `/api/pub?topic=${target}`
      : `/api/queue/${target}`

    const ttlInput = ttlSeconds.trim()
    const parsedTtl = ttlInput ? Number(ttlInput) : null
    const ttlValue = parsedTtl && parsedTtl > 0 ? parsedTtl : null
    const requestBody = ttlValue
      ? JSON.stringify({ payload: msgInput, ttl_seconds: ttlValue })
      : JSON.stringify({ payload: msgInput })
    await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: requestBody,
    })
    setMsgInput('')
  }

  const subscriberEntries = Object.entries(metrics.topics.entries)
    .map(([name, entry]) => [name, entry.subscribers] as const)
    .sort((a, b) => b[1] - a[1])
  const queueEntries = Object.entries(metrics.queues.entries)
    .map(([name, entry]) => [name, entry.depth] as const)
    .sort((a, b) => b[1] - a[1])
  const expiredEntries = stats.expired.slice().reverse().slice(0, 10)

  const totalSubscribers = subscriberEntries.reduce((sum, [, value]) => sum + value, 0)
  const totalQueueDepth = queueEntries.reduce((sum, [, value]) => sum + value, 0)

  const maxSubscribers = Math.max(1, ...subscriberEntries.map(([, value]) => value))
  const maxQueueDepth = Math.max(1, ...queueEntries.map(([, value]) => value))

  const formatRemaining = (expiresAt: number | null) => {
    if (!expiresAt) return 'No TTL'
    const remainingMs = Math.max(0, expiresAt - Date.now())
    const remainingSeconds = Math.ceil(remainingMs / 1000)
    if (remainingSeconds < 60) return `${remainingSeconds}s`
    const minutes = Math.floor(remainingSeconds / 60)
    const seconds = remainingSeconds % 60
    return `${minutes}m ${seconds}s`
  }

  const formatTtlLabel = (meta: EntryMeta, expiresAt: number | null) => {
    if (!meta.ttl_seconds) return 'No TTL'
    return `TTL ${meta.ttl_seconds}s • ${formatRemaining(expiresAt)}`
  }

  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString()
  }

  const formatAge = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000)
    if (seconds < 60) return `${seconds}s ago`
    const minutes = Math.floor(seconds / 60)
    if (minutes < 60) return `${minutes}m ago`
    const hours = Math.floor(minutes / 60)
    return `${hours}h ago`
  }

  return (
    <div className="app-container">
      <header className="app-header">
        <div className="logo-container">
          <img src="/logo.png" alt="Bus9" />
        </div>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
            <h1>Bus9</h1>
            {version && (
              <span className="version-badge">v{version}</span>
            )}
          </div>
          <span className="subtitle">Message Broker</span>
        </div>
      </header>

      {/* Overview Cards */}
      <div className="overview-grid">
        <div className="metric-card">
          <div className="metric-label">Active Topics</div>
          <div className="metric-value">{metrics.topics.active}</div>
          <div className="metric-detail">{totalSubscribers} subscribers</div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Active Queues</div>
          <div className="metric-value">{metrics.queues.active}</div>
          <div className="metric-detail">{totalQueueDepth} messages</div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Top Topic</div>
          {subscriberEntries.length === 0 ? (
            <div className="metric-detail">No topics</div>
          ) : (
            <>
              <div className="metric-value-small">{subscriberEntries[0][0]}</div>
              <div className="metric-detail">{subscriberEntries[0][1]} subscribers</div>
            </>
          )}
        </div>

        <div className="metric-card">
          <div className="metric-label">Top Queue</div>
          {queueEntries.length === 0 ? (
            <div className="metric-detail">No queues</div>
          ) : (
            <>
              <div className="metric-value-small">{queueEntries[0][0]}</div>
              <div className="metric-detail">{queueEntries[0][1]} messages</div>
            </>
          )}
        </div>
      </div>

      {/* Main Content Grid */}
      <div className="content-grid">
        {/* Topics Section */}
        <div className="section-card">
          <div className="section-header">
            <Activity size={18} />
            <h2>Topics</h2>
            <span className="section-count">{Object.keys(stats.topics).length}</span>
          </div>
          
          {Object.keys(stats.topics).length === 0 ? (
            <div className="empty-state">No active topics</div>
          ) : (
            <div className="items-list">
              {Object.entries(stats.topics).map(([name, info]) => (
                <div key={name} className="item-row">
                  <div className="item-main">
                    <Radio size={14} />
                    <span className="item-name">{name}</span>
                    <span className="item-badge">{info.subscribers}</span>
                  </div>
                  <div className="item-meta">{formatTtlLabel(info.meta, info.expires_at)}</div>
                </div>
              ))}
            </div>
          )}

          {subscriberEntries.length > 0 && (
            <div className="subsection">
              <div className="subsection-title">Subscribers by Topic</div>
              <div className="bars-list">
                {subscriberEntries.slice(0, 8).map(([name, value]) => (
                  <div key={name} className="bar-item">
                    <div className="bar-label">
                      <span>{name}</span>
                      <span>{value}</span>
                    </div>
                    <div className="bar-track">
                      <div 
                        className="bar-fill" 
                        style={{ width: `${(value / maxSubscribers) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Queues Section */}
        <div className="section-card">
          <div className="section-header">
            <Boxes size={18} />
            <h2>Queues</h2>
            <span className="section-count">{Object.keys(stats.queues).length}</span>
          </div>
          
          {Object.keys(stats.queues).length === 0 ? (
            <div className="empty-state">No active queues</div>
          ) : (
            <div className="items-list">
              {Object.entries(stats.queues).map(([name, info]) => (
                <div key={name} className="item-row">
                  <div className="item-main">
                    <Server size={14} />
                    <span className="item-name">{name}</span>
                    <span className={`item-badge ${info.depth > 0 ? 'active' : ''}`}>
                      {info.depth}
                    </span>
                  </div>
                  <div className="item-meta">{formatTtlLabel(info.meta, info.expires_at)}</div>
                </div>
              ))}
            </div>
          )}

          {queueEntries.length > 0 && (
            <div className="subsection">
              <div className="subsection-title">Queue Depth</div>
              <div className="bars-list">
                {queueEntries.slice(0, 8).map(([name, value]) => (
                  <div key={name} className="bar-item">
                    <div className="bar-label">
                      <span>{name}</span>
                      <span>{value}</span>
                    </div>
                    <div className="bar-track">
                      <div 
                        className="bar-fill queue" 
                        style={{ width: `${(value / maxQueueDepth) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Expired Section */}
        {expiredEntries.length > 0 && (
          <div className="section-card">
            <div className="section-header">
              <TrendingUp size={18} />
              <h2>Recently Expired</h2>
              <span className="section-count">{expiredEntries.length}</span>
            </div>
            <div className="expired-list">
              {expiredEntries.map(entry => (
                <div key={`${entry.kind}-${entry.name}-${entry.expired_at}`} className="expired-item">
                  <div className="expired-name">{entry.name}</div>
                  <div className="expired-meta">
                    <span>{entry.kind}</span>
                    <span>•</span>
                    <span>{formatTime(entry.expired_at)}</span>
                    <span>•</span>
                    <span>{formatAge(entry.expired_at)}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Publish Control */}
        <div className="section-card">
          <h2>Publish Message</h2>
          <div className="mode-switch">
            <button 
              className={mode === 'pub' ? 'active' : ''} 
              onClick={() => setMode('pub')}
            >
              Topic
            </button>
            <button 
              className={mode === 'queue' ? 'active' : ''} 
              onClick={() => setMode('queue')}
            >
              Queue
            </button>
          </div>

          <div className="form-group">
            <label>Target {mode === 'pub' ? 'Topic' : 'Queue'}</label>
            <input 
              value={target} 
              onChange={e => setTarget(e.target.value)} 
              placeholder={mode === 'pub' ? 'e.g. news-feed' : 'e.g. worker-jobs'} 
            />
          </div>

          <div className="form-group">
            <label>Payload</label>
            <input 
              value={msgInput} 
              onChange={e => setMsgInput(e.target.value)} 
              placeholder="Type a message..." 
              onKeyDown={e => e.key === 'Enter' && handleSend()} 
            />
          </div>

          <div className="form-group">
            <label>TTL (seconds, optional)</label>
            <input
              type="number"
              min="1"
              value={ttlSeconds}
              onChange={e => setTtlSeconds(e.target.value)}
              placeholder="e.g. 120"
            />
          </div>

          <button className="send-button" onClick={handleSend}>
            <Send size={16} />
            Send
          </button>
        </div>

        {/* Live Monitor */}
        <div className="section-card monitor-card">
          <div className="section-header">
            <h2>Live Monitor</h2>
            <div className="monitor-control">
              <span>Topic:</span>
              <input 
                className="topic-input" 
                value={subTopic} 
                onChange={e => setSubTopic(e.target.value)} 
              />
            </div>
          </div>

          <div className="monitor-log">
            {logs.length === 0 ? (
              <div className="empty-state">Waiting for messages...</div>
            ) : (
              logs.map(log => (
                <div key={log.id} className="log-entry">
                  <div className="log-header">
                    <span className="log-time">{formatTime(log.timestamp)}</span>
                    <span className="log-id">{log.id.slice(0, 8)}</span>
                  </div>
                  <div className="log-payload">{log.payload}</div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

export default App
