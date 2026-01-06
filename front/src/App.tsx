import { useEffect, useState, useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Activity, Radio, Server, Boxes, Send } from 'lucide-react'
import './index.css'

interface Stats {
  topics: Record<string, number>
  queues: Record<string, number>
}

interface LogMessage {
  id: string
  payload: string
  timestamp: number
}

function App() {
  const [stats, setStats] = useState<Stats>({ topics: {}, queues: {} })
  const [msgInput, setMsgInput] = useState('')
  const [target, setTarget] = useState('')
  const [mode, setMode] = useState<'pub' | 'queue'>('pub')
  const [logs, setLogs] = useState<LogMessage[]>([])
  const [subTopic, setSubTopic] = useState('test')
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

    await fetch(url, { method: 'POST', body: msgInput })
    setMsgInput('')
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

        {/* Topology Stats */}
        <div className="panel" style={{ gridColumn: 'span 2', display: 'flex', gap: '2rem', flexDirection: 'row', flexWrap: 'wrap' }}>
          <div style={{ flex: 1, minWidth: 300 }}>
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: 0 }}><Activity size={20} /> Active Topics</h2>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem', marginTop: '1rem' }}>
              {Object.entries(stats.topics).length === 0 && <span style={{ opacity: 0.5 }}>No active topics</span>}
              {Object.entries(stats.topics).map(([name, count]) => (
                <motion.div layoutId={`topic-${name}`} key={name} style={{ background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: 20, border: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <Radio size={14} color="var(--accent-color)" />
                  <span>{name}</span>
                  <span style={{ background: 'var(--accent-color)', color: 'black', borderRadius: 10, padding: '0 6px', fontSize: '0.8em', fontWeight: 'bold' }}>{count}</span>
                </motion.div>
              ))}
            </div>
          </div>

          <div style={{ width: 1, background: 'var(--border-color)' }} />

          <div style={{ flex: 1, minWidth: 300 }}>
            <h2 style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: 0 }}><Boxes size={20} /> Active Queues</h2>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: '1rem', marginTop: '1rem' }}>
              {Object.entries(stats.queues).length === 0 && <span style={{ opacity: 0.5 }}>No queues</span>}
              {Object.entries(stats.queues).map(([name, count]) => (
                <motion.div layoutId={`queue-${name}`} key={name} style={{ background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: 6, border: '1px solid var(--border-color)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <Server size={14} color={count > 0 ? 'var(--success-color)' : 'gray'} />
                  <span>{name}</span>
                  <span style={{ background: count > 0 ? 'var(--success-color)' : 'gray', color: 'black', borderRadius: 10, padding: '0 6px', fontSize: '0.8em', fontWeight: 'bold' }}>{count}</span>
                </motion.div>
              ))}
            </div>
          </div>
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
