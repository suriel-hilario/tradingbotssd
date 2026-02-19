<template>
  <div class="login-wrap">
    <div class="login-box">
      <h2>ClawBot Dashboard</h2>
      <form @submit.prevent="submit">
        <input
          v-model="token"
          type="password"
          placeholder="Bearer token"
          autocomplete="current-password"
        />
        <button type="submit">Login</button>
        <p v-if="error" class="error">{{ error }}</p>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

const token = ref('')
const error = ref('')
const router = useRouter()

async function submit() {
  error.value = ''
  // Verify token against /healthz (no auth required) then check portfolio
  const resp = await fetch('/api/portfolio', {
    headers: { Authorization: `Bearer ${token.value}` },
  })
  if (resp.ok) {
    sessionStorage.setItem('dashboard_token', token.value)
    router.push('/overview')
  } else {
    error.value = 'Invalid token. Please try again.'
  }
}
</script>

<style scoped>
.login-wrap {
  display: flex; align-items: center; justify-content: center;
  min-height: 100vh; background: #0f0f0f;
}
.login-box {
  background: #1a1a1a; border: 1px solid #333; border-radius: 8px;
  padding: 40px; width: 360px;
}
h2 { margin-bottom: 24px; text-align: center; }
input {
  display: block; width: 100%; padding: 10px; margin-bottom: 12px;
  background: #0f0f0f; border: 1px solid #444; color: #e0e0e0;
  border-radius: 4px; font-size: 1rem;
}
button {
  display: block; width: 100%; padding: 10px; background: #2980b9;
  color: #fff; border: none; border-radius: 4px; font-size: 1rem; cursor: pointer;
}
.error { color: #e74c3c; margin-top: 10px; font-size: 0.9rem; }
</style>
