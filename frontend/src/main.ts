import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import Overview from './views/Overview.vue'
import Operations from './views/Operations.vue'
import StrategyConfig from './views/StrategyConfig.vue'
import Performance from './views/Performance.vue'
import Login from './views/Login.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', component: Login },
    { path: '/', redirect: '/overview' },
    { path: '/overview', component: Overview, meta: { requiresAuth: true } },
    { path: '/operations', component: Operations, meta: { requiresAuth: true } },
    { path: '/config', component: StrategyConfig, meta: { requiresAuth: true } },
    { path: '/performance', component: Performance, meta: { requiresAuth: true } },
  ],
})

router.beforeEach((to) => {
  const token = sessionStorage.getItem('dashboard_token')
  if (to.meta.requiresAuth && !token) {
    return '/login'
  }
})

createApp(App).use(router).mount('#app')
