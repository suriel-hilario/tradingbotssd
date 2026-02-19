<template>
  <div class="container">
    <h1>Performance</h1>

    <div v-if="!data || data.trade_count === 0">
      <div class="card"><p style="color:#888">No completed trades yet.</p></div>
    </div>
    <template v-else>
      <!-- Stats card -->
      <div class="card stats-grid">
        <div><label>Total PnL</label><span :style="{color: data.total_pnl_usd >= 0 ? '#2ecc71' : '#e74c3c'}">{{ data.total_pnl_usd.toFixed(2) }} USD</span></div>
        <div><label>Win Rate</label><span>{{ (data.win_rate * 100).toFixed(1) }}%</span></div>
        <div><label>Trades</label><span>{{ data.trade_count }}</span></div>
        <div><label>Max Drawdown</label><span style="color:#e74c3c">{{ (data.max_drawdown_pct * 100).toFixed(2) }}%</span></div>
      </div>

      <!-- Equity curve -->
      <div class="card">
        <h3>Equity Curve</h3>
        <canvas ref="chartCanvas" height="200"></canvas>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { Chart, LineController, LineElement, PointElement, LinearScale, TimeScale, Tooltip } from 'chart.js'

Chart.register(LineController, LineElement, PointElement, LinearScale, Tooltip)

const data = ref<any>(null)
const chartCanvas = ref<HTMLCanvasElement | null>(null)
let chart: Chart | null = null

async function fetchPerformance() {
  const resp = await fetch('/api/performance', {
    headers: { Authorization: `Bearer ${sessionStorage.getItem('dashboard_token')}` },
  })
  if (resp.ok) data.value = await resp.json()
}

function renderChart() {
  if (!chartCanvas.value || !data.value?.equity_curve?.length) return
  if (chart) chart.destroy()
  const labels = data.value.equity_curve.map((p: any) => p.timestamp)
  const values = data.value.equity_curve.map((p: any) => p.value)
  chart = new Chart(chartCanvas.value, {
    type: 'line',
    data: {
      labels,
      datasets: [{
        label: 'Equity (USD)',
        data: values,
        borderColor: '#2980b9',
        backgroundColor: 'rgba(41,128,185,0.1)',
        tension: 0.3,
        pointRadius: 2,
      }],
    },
    options: {
      responsive: true,
      plugins: { tooltip: { enabled: true } },
      scales: {
        x: { ticks: { color: '#888', maxTicksLimit: 10 }, grid: { color: '#222' } },
        y: { ticks: { color: '#888' }, grid: { color: '#222' } },
      },
    },
  })
}

onMounted(async () => {
  await fetchPerformance()
  renderChart()
})
watch(data, renderChart)
</script>

<style scoped>
.stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; }
.stats-grid label { display: block; color: #888; font-size: 0.8rem; margin-bottom: 4px; }
.stats-grid span { font-size: 1.3rem; font-weight: bold; }
</style>
