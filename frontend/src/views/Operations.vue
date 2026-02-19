<template>
  <div class="container">
    <h1>Operations</h1>

    <!-- Live log terminal -->
    <div class="card">
      <h3>Live Logs</h3>
      <pre ref="logEl" class="log-terminal">{{ logLines.join('\n') }}</pre>
    </div>

    <!-- Trade history -->
    <div class="card">
      <h3>Trade History</h3>
      <input v-model="pairFilter" placeholder="Filter by pair (e.g. BTCUSDT)" style="margin-bottom:12px;padding:6px;background:#0f0f0f;border:1px solid #444;color:#e0e0e0;width:240px" />
      <table>
        <thead>
          <tr><th>Time</th><th>Pair</th><th>Side</th><th>Entry</th><th>Exit</th><th>Qty</th><th>PnL USD</th></tr>
        </thead>
        <tbody>
          <tr v-for="t in trades" :key="t.id">
            <td>{{ t.closed_at }}</td>
            <td>{{ t.pair }}</td>
            <td>{{ t.side }}</td>
            <td>{{ t.entry_price?.toFixed(4) }}</td>
            <td>{{ t.exit_price?.toFixed(4) }}</td>
            <td>{{ t.quantity }}</td>
            <td :style="{ color: t.pnl_usd >= 0 ? '#2ecc71' : '#e74c3c' }">
              {{ t.pnl_usd?.toFixed(2) }}
            </td>
          </tr>
        </tbody>
      </table>
      <div style="margin-top:12px;display:flex;gap:8px">
        <button @click="prevPage" :disabled="page <= 1">← Prev</button>
        <span>Page {{ page }}</span>
        <button @click="nextPage" :disabled="trades.length < limit">Next →</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick } from 'vue'

const logLines = ref<string[]>([])
const logEl = ref<HTMLPreElement | null>(null)
const MAX_LINES = 500
let ws: WebSocket | null = null
let autoScroll = true

const trades = ref<any[]>([])
const page = ref(1)
const limit = 50
const pairFilter = ref('')

function connectWs() {
  const token = sessionStorage.getItem('dashboard_token')
  ws = new WebSocket(`ws://${location.host}/ws/logs?token=${token}`)
  ws.onmessage = async (e) => {
    logLines.value.push(e.data)
    if (logLines.value.length > MAX_LINES) logLines.value.shift()
    if (autoScroll) {
      await nextTick()
      logEl.value?.scrollTo(0, logEl.value.scrollHeight)
    }
  }
  ws.onclose = () => setTimeout(connectWs, 3000) // reconnect
}

function onScroll() {
  const el = logEl.value
  if (!el) return
  autoScroll = el.scrollTop + el.clientHeight >= el.scrollHeight - 10
}

async function fetchTrades() {
  const qs = new URLSearchParams({ page: String(page.value), limit: String(limit) })
  if (pairFilter.value) qs.set('pair', pairFilter.value)
  const resp = await fetch(`/api/trades?${qs}`, {
    headers: { Authorization: `Bearer ${sessionStorage.getItem('dashboard_token')}` },
  })
  if (resp.ok) {
    const json = await resp.json()
    trades.value = json.trades
  }
}

function prevPage() { if (page.value > 1) { page.value--; fetchTrades() } }
function nextPage() { page.value++; fetchTrades() }

watch(pairFilter, () => { page.value = 1; fetchTrades() })

onMounted(() => {
  connectWs()
  logEl.value?.addEventListener('scroll', onScroll)
  fetchTrades()
})
onUnmounted(() => { ws?.close() })
</script>

<style scoped>
.log-terminal {
  background: #0a0a0a; border: 1px solid #333; padding: 12px;
  height: 300px; overflow-y: auto; font-size: 0.8rem;
  font-family: 'Courier New', monospace; white-space: pre-wrap; word-break: break-all;
}
button { padding: 6px 14px; background: #2c2c2c; color: #e0e0e0; border: 1px solid #444; cursor: pointer; border-radius: 4px; }
button:disabled { opacity: 0.4; cursor: default; }
</style>
