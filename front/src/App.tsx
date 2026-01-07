import { useEffect, useState, useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Activity, Radio, Server, Boxes, Send, BarChart3 } from 'lucide-react'
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
  const [requestHistory, setRequestHistory] = useState<number[]>([])
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
    let active = true

    const fetchMetrics = async () => {
      try {
        const response = await fetch('/api/metrics')
        if (!response.ok) return
        const data: MetricsResponse = await response.json()
        if (!active) return
        setMetrics(data)
        const totalRequests = Object.values(data.requests).reduce((sum, value) => sum + value, 0)
        setRequestHistory(prev => [...prev, totalRequests].slice(-24))
      } catch (error) {
        console.error(error)
      }
    }

    fetchMetrics()
    const intervalId = window.setInterval(fetchMetrics, 1000)
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

  const requestEntries = Object.entries(metrics.requests).sort((a, b) => b[1] - a[1])
  const subscriberEntries = Object.entries(metrics.topics.entries)
    .map(([name, entry]) => [name, entry.subscribers] as const)
    .sort((a, b) => b[1] - a[1])
  const queueEntries = Object.entries(metrics.queues.entries)
    .map(([name, entry]) => [name, entry.depth] as const)
    .sort((a, b) => b[1] - a[1])
  const expiredEntries = stats.expired.slice().reverse().slice(0, 6)

  const totalRequests = requestEntries.reduce((sum, [, value]) => sum + value, 0)
  const totalSubscribers = subscriberEntries.reduce((sum, [, value]) => sum + value, 0)
  const totalQueueDepth = queueEntries.reduce((sum, [, value]) => sum + value, 0)


  const maxSubscribers = Math.max(1, ...subscriberEntries.map(([, value]) => value))
  const maxQueueDepth = Math.max(1, ...queueEntries.map(([, value]) => value))
  const maxHistory = Math.max(1, ...requestHistory)

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
    return `TTL ${meta.ttl_seconds}s • expires in ${formatRemaining(expiresAt)}`
  }

  return (
    <div style={{ padding: '2rem', maxWidth: '1200px', margin: '0 auto' }}>
      <header style={{ marginBottom: '3rem', display: 'flex', alignItems: 'center', gap: '1rem' }}>
        <div style={{ width: 40, height: 40, borderRadius: 8, overflow: 'hidden', boxShadow: '0 0 20px var(--accent-glow)' }}>
          <img src="/logo.png" style={{ width: '100%', height: '100%', objectFit: 'cover' }} alt="Bus9 Logo" />
        </div>
        <div>
          <h1 style={{ margin: 0, fontSize: '2rem' }}>Bus9</h1>
          <span style={{ color: 'var(--text-secondary)' }}>Minimalist Message Broker</span>
        </div>
      </header>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '2rem' }}>

        {/* Metrics */}
        <div className="panel" style={{ gridColumn: 'span 2', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
          <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: 0 }}>
            <BarChart3 size={20} /> Live Metrics
          </h2>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '1rem' }}>
            <div style={{ border: '1px solid var(--border-color)', borderRadius: 12, padding: '1rem', background: 'rgba(255,255,255,0.02)' }}>
              <div style={{ fontSize: '0.85em', color: 'var(--text-secondary)' }}>Total Requests</div>
              <div style={{ fontSize: '1.6rem', fontWeight: 600 }}>{totalRequests.toLocaleString()}</div>
              <div style={{ display: 'flex', alignItems: 'flex-end', gap: 4, height: 40, marginTop: '0.75rem' }}>
                {requestHistory.map((value, index) => (
                  <div
                    key={`${value}-${index}`}
                    style={{
                      width: 6,
                      height: `${(value / maxHistory) * 40}px`,
                      borderRadius: 6,
                      background: 'var(--accent-color)',
                      opacity: 0.6,
                    }}
                  />
                ))}
                {requestHistory.length === 0 && (
                  <div style={{ color: 'var(--text-secondary)', fontSize: '0.8em' }}>Waiting for traffic...</div>
                )}
              </div>
            </div>

            <div style={{ border: '1px solid var(--border-color)', borderRadius: 12, padding: '1rem', background: 'rgba(255,255,255,0.02)' }}>
              <div style={{ fontSize: '0.85em', color: 'var(--text-secondary)' }}>Active Topics</div>
              <div style={{ fontSize: '1.6rem', fontWeight: 600 }}>{metrics.topics.active}</div>
              <div style={{ marginTop: '0.5rem', color: 'var(--text-secondary)' }}>{totalSubscribers} total subscribers</div>
            </div>

            <div style={{ border: '1px solid var(--border-color)', borderRadius: 12, padding: '1rem', background: 'rgba(255,255,255,0.02)' }}>
              <div style={{ fontSize: '0.85em', color: 'var(--text-secondary)' }}>Active Queues</div>
              <div style={{ fontSize: '1.6rem', fontWeight: 600 }}>{metrics.queues.active}</div>
              <div style={{ marginTop: '0.5rem', color: 'var(--text-secondary)' }}>{totalQueueDepth} messages queued</div>
            </div>

            <div style={{ border: '1px solid var(--border-color)', borderRadius: 12, padding: '1rem', background: 'rgba(255,255,255,0.02)' }}>
              <div style={{ fontSize: '0.85em', color: 'var(--text-secondary)' }}>Top Endpoint</div>
              {requestEntries.length === 0 ? (
                <div style={{ color: 'var(--text-secondary)', marginTop: '0.5rem' }}>No requests yet</div>
              ) : (
                <>
                  <div style={{ fontSize: '1.4rem', fontWeight: 600 }}>{requestEntries[0][1].toLocaleString()}</div>
                  <div style={{ color: 'var(--text-secondary)' }}>{requestEntries[0][0]}</div>
                </>
              )}
            </div>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: '1rem' }}>


            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
              <h3 style={{ margin: 0, fontSize: '1rem' }}>Subscribers by Topic</h3>
              {subscriberEntries.length === 0 && <span style={{ opacity: 0.6 }}>No subscribers yet</span>}
              {subscriberEntries.map(([name, value]) => (
                <div key={name} style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85em', color: 'var(--text-secondary)' }}>
                    <span>{name}</span>
                    <span>{value}</span>
                  </div>
                  <div style={{ height: 6, background: 'rgba(255,255,255,0.08)', borderRadius: 999 }}>
                    <div style={{ height: '100%', width: `${(value / maxSubscribers) * 100}%`, background: 'var(--success-color)', borderRadius: 999 }} />
                  </div>
                </div>
              ))}
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
              <h3 style={{ margin: 0, fontSize: '1rem' }}>Queue Depth</h3>
              {queueEntries.length === 0 && <span style={{ opacity: 0.6 }}>No queues yet</span>}
              {queueEntries.map(([name, value]) => (
                <div key={name} style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85em', color: 'var(--text-secondary)' }}>
                    <span>{name}</span>
                    <span>{value}</span>
                  </div>
                  <div style={{ height: 6, background: 'rgba(255,255,255,0.08)', borderRadius: 999 }}>
                    <div style={{ height: '100%', width: `${(value / maxQueueDepth) * 100}%`, background: 'var(--accent-color)', borderRadius: 999 }} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Topology Stats */}
        <div className="panel" style={{ gridColumn: 'span 2', display: 'flex', gap: '2rem', flexDirection: 'row', flexWrap: 'wrap' }}>
          <div style={{ flex: 1, minWidth: 300 }}>
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: 0 }}><Activity size={20} /> Active Topics</h2>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem', marginTop: '1rem' }}>
              {Object.entries(stats.topics).length === 0 && <span style={{ opacity: 0.5 }}>No active topics</span>}
              {Object.entries(stats.topics).map(([name, info]) => (
                <motion.div layoutId={`topic-${name}`} key={name} style={{ background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: 20, border: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Radio size={14} color="var(--accent-color)" />
                    <span>{name}</span>
                    <span style={{ background: 'var(--accent-color)', color: 'black', borderRadius: 10, padding: '0 6px', fontSize: '0.8em', fontWeight: 'bold' }}>{info.subscribers}</span>
                  </div>
                  <span style={{ fontSize: '0.75em', color: 'var(--text-secondary)' }}>{formatTtlLabel(info.meta, info.expires_at)}</span>
                </motion.div>
              ))}
            </div>
          </div>

          <div style={{ width: 1, background: 'var(--border-color)' }} />

          <div style={{ flex: 1, minWidth: 300 }}>
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: 0 }}><Boxes size={20} /> Active Queues</h2>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem', marginTop: '1rem' }}>
              {Object.entries(stats.queues).length === 0 && <span style={{ opacity: 0.5 }}>No queues</span>}
              {Object.entries(stats.queues).map(([name, info]) => (
                <motion.div layoutId={`queue-${name}`} key={name} style={{ background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: 6, border: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <Server size={14} color={info.depth > 0 ? 'var(--success-color)' : 'gray'} />
                    <span>{name}</span>
                    <span style={{ background: info.depth > 0 ? 'var(--success-color)' : 'gray', color: 'black', borderRadius: 10, padding: '0 6px', fontSize: '0.8em', fontWeight: 'bold' }}>{info.depth}</span>
                  </div>
                  <span style={{ fontSize: '0.75em', color: 'var(--text-secondary)' }}>{formatTtlLabel(info.meta, info.expires_at)}</span>
                </motion.div>
              ))}
            </div>
          </div>
        </div>

        <div className="panel" style={{ gridColumn: 'span 2' }}>
          <h2 style={{ marginTop: 0 }}>Recently Expired</h2>
          {expiredEntries.length === 0 ? (
            <span style={{ opacity: 0.6 }}>No expirations yet</span>
          ) : (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '0.75rem' }}>
              {expiredEntries.map(entry => (
                <div key={`${entry.kind}-${entry.name}-${entry.expired_at}`} style={{ border: '1px solid var(--border-color)', borderRadius: 10, padding: '0.75rem', background: 'rgba(255,255,255,0.02)' }}>
                  <div style={{ fontWeight: 600 }}>{entry.name}</div>
                  <div style={{ fontSize: '0.8em', color: 'var(--text-secondary)' }}>{entry.kind} expired</div>
                  <div style={{ fontSize: '0.75em', color: 'var(--text-secondary)' }}>{new Date(entry.expired_at).toLocaleTimeString()}</div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Controls */}
        <div className="panel">
          <h2 style={{ marginTop: 0 }}>Publish Message</h2>
          <div style={{ display: 'flex', gap: '1rem', marginBottom: '1rem' }}>
            <button style={{ flex: 1, borderColor: mode === 'pub' ? 'var(--accent-color)' : '' }} onClick={() => setMode('pub')}>Topic</button>
            <button style={{ flex: 1, borderColor: mode === 'queue' ? 'var(--accent-color)' : '' }} onClick={() => setMode('queue')}>Queue</button>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div>
              <label style={{ display: 'block', marginBottom: '0.5rem', fontSize: '0.9em', color: 'var(--text-secondary)' }}>Target {mode === 'pub' ? 'Topic' : 'Queue'}</label>
              <input value={target} onChange={e => setTarget(e.target.value)} placeholder={mode === 'pub' ? 'e.g. news-feed' : 'e.g. worker-jobs'} />
            </div>
            <div>
              <label style={{ display: 'block', marginBottom: '0.5rem', fontSize: '0.9em', color: 'var(--text-secondary)' }}>Payload</label>
              <input value={msgInput} onChange={e => setMsgInput(e.target.value)} placeholder="Type a message..." onKeyDown={e => e.key === 'Enter' && handleSend()} />
            </div>
            <div>
              <label style={{ display: 'block', marginBottom: '0.5rem', fontSize: '0.9em', color: 'var(--text-secondary)' }}>Inactivity TTL (seconds, optional)</label>
              <input
                type="number"
                min="1"
                value={ttlSeconds}
                onChange={e => setTtlSeconds(e.target.value)}
                placeholder="e.g. 120"
              />
            </div>
            <button onClick={handleSend} style={{ background: 'var(--accent-color)', color: 'black', borderColor: 'transparent', marginTop: '0.5rem', display: 'flex', justifyContent: 'center', gap: '0.5rem' }}>
              <Send size={18} /> Send
            </button>
          </div>
        </div>

        {/* Live Stream */}
        <div className="panel" style={{ height: 400, display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
            <h2 style={{ marginTop: 0 }}>Live Monitor</h2>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <span style={{ fontSize: '0.8em', color: 'var(--text-secondary)' }}>Watching:</span>
              <input style={{ width: 120, padding: '0.2rem 0.5rem' }} value={subTopic} onChange={e => setSubTopic(e.target.value)} />
            </div>
          </div>

          <div style={{ flex: 1, overflowY: 'auto', background: '#000', borderRadius: 8, padding: '0.5rem', fontFamily: 'monospace', fontSize: '0.9em' }}>
            <AnimatePresence initial={false}>
              {logs.map(log => (
                <motion.div
                  key={log.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0 }}
                  style={{ marginBottom: '0.5rem', borderBottom: '1px solid #222', paddingBottom: '0.5rem' }}
                >
                  <div style={{ color: 'var(--text-secondary)', fontSize: '0.8em' }}>{new Date(log.timestamp).toLocaleTimeString()} <span style={{ color: 'var(--accent-color)' }}>{log.id.slice(0, 8)}</span></div>
                  <div style={{ color: 'white' }}>{log.payload}</div>
                </motion.div>
              ))}
              {logs.length === 0 && <div style={{ color: '#444', textAlign: 'center', marginTop: '2rem' }}>Waiting for messages...</div>}
            </AnimatePresence>
          </div>
        </div>

      </div>
    </div>
  )
}

export default App
