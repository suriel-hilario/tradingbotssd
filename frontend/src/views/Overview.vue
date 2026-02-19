<template>
  <div class="container">
    <h1>Overview</h1>
    <div class="card">
      <h3>Portfolio</h3>
      <p>Open positions: {{ data?.total_open ?? '—' }}</p>
    </div>
    <div class="card" v-if="data?.positions?.length">
      <h3>Open Positions</h3>
      <table>
        <thead>
          <tr><th>Pair</th><th>Side</th><th>Entry</th><th>Qty</th><th>Mode</th></tr>
        </thead>
        <tbody>
          <tr v-for="p in data.positions" :key="p.id">
            <td>{{ p.pair }}</td>
            <td>{{ p.side }}</td>
            <td>{{ p.entry_price.toFixed(4) }}</td>
            <td>{{ p.quantity }}</td>
            <td>{{ p.mode }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-else-if="data" style="color:#888">No open positions.</p>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

const data = ref<any>(null)
let timer: ReturnType<typeof setInterval>

async function fetchPortfolio() {
  const resp = await fetch('/api/portfolio', {
    headers: { Authorization: `Bearer ${sessionStorage.getItem('dashboard_token')}` },
  })
  if (resp.ok) data.value = await resp.json()
}

onMounted(() => {
  fetchPortfolio()
  timer = setInterval(fetchPortfolio, 5000)
})
onUnmounted(() => clearInterval(timer))
</script>
