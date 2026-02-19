<template>
  <div id="app">
    <nav v-if="isAuthed" class="navbar">
      <span class="brand">🦞 ClawBot</span>
      <router-link to="/overview">Overview</router-link>
      <router-link to="/operations">Operations</router-link>
      <router-link to="/config">Config</router-link>
      <router-link to="/performance">Performance</router-link>
      <button @click="logout" class="logout">Logout</button>
    </nav>
    <router-view />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const isAuthed = computed(() => !!sessionStorage.getItem('dashboard_token'))

function logout() {
  sessionStorage.removeItem('dashboard_token')
  router.push('/login')
}
</script>

<style>
.navbar {
  display: flex; gap: 16px; align-items: center;
  padding: 12px 24px; background: #1a1a1a; border-bottom: 1px solid #333;
}
.brand { font-weight: bold; font-size: 1.1rem; margin-right: auto; }
.navbar a { color: #aaa; text-decoration: none; }
.navbar a.router-link-active { color: #fff; }
.logout { background: #c0392b; color: #fff; border: none; padding: 6px 12px; cursor: pointer; border-radius: 4px; }
.container { padding: 24px; }
table { width: 100%; border-collapse: collapse; }
th, td { padding: 8px 12px; border-bottom: 1px solid #333; text-align: left; }
th { color: #aaa; font-weight: 500; }
.card { background: #1a1a1a; border: 1px solid #333; border-radius: 8px; padding: 20px; margin-bottom: 16px; }
</style>
