<template>
  <div class="container">
    <h1>Strategy & Config</h1>
    <div class="card">
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
        <h3>Current Configuration</h3>
        <button v-if="!editing" @click="startEdit">Edit</button>
        <div v-else style="display:flex;gap:8px">
          <button @click="applyConfig" class="apply-btn">Apply</button>
          <button @click="cancelEdit">Cancel</button>
        </div>
      </div>

      <pre v-if="!editing" class="config-view">{{ configText }}</pre>
      <textarea v-else v-model="editBuffer" class="config-edit" rows="20" />
      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
      <p v-if="successMsg" class="success">{{ successMsg }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const configText = ref('')
const editBuffer = ref('')
const editing = ref(false)
const errorMsg = ref('')
const successMsg = ref('')

async function fetchConfig() {
  const resp = await fetch('/api/config', {
    headers: { Authorization: `Bearer ${sessionStorage.getItem('dashboard_token')}` },
  })
  if (resp.ok) {
    const json = await resp.json()
    configText.value = JSON.stringify(json, null, 2)
  }
}

function startEdit() {
  editBuffer.value = configText.value
  editing.value = true
  errorMsg.value = ''
  successMsg.value = ''
}

function cancelEdit() {
  editing.value = false
  errorMsg.value = ''
}

async function applyConfig() {
  errorMsg.value = ''
  successMsg.value = ''
  let body: any
  try {
    body = JSON.parse(editBuffer.value)
  } catch {
    errorMsg.value = 'Invalid JSON: ' + String(Error)
    return
  }
  const resp = await fetch('/api/config', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${sessionStorage.getItem('dashboard_token')}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  })
  if (resp.ok) {
    successMsg.value = 'Configuration applied successfully.'
    configText.value = editBuffer.value
    editing.value = false
    setTimeout(() => { successMsg.value = '' }, 3000)
  } else {
    const err = await resp.json().catch(() => ({ error: 'Unknown error' }))
    errorMsg.value = err.error || JSON.stringify(err)
  }
}

onMounted(fetchConfig)
</script>

<style scoped>
.config-view {
  background: #0a0a0a; border: 1px solid #333; padding: 12px;
  font-family: monospace; font-size: 0.85rem; white-space: pre-wrap; overflow-x: auto;
}
.config-edit {
  width: 100%; background: #0a0a0a; border: 1px solid #444;
  color: #e0e0e0; padding: 12px; font-family: monospace; font-size: 0.85rem; resize: vertical;
}
button { padding: 6px 14px; background: #2c2c2c; color: #e0e0e0; border: 1px solid #444; cursor: pointer; border-radius: 4px; }
.apply-btn { background: #27ae60; border-color: #27ae60; }
.error { color: #e74c3c; margin-top: 8px; }
.success { color: #2ecc71; margin-top: 8px; }
</style>
