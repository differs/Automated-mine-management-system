<script setup lang="ts">
import { useAuthStore } from '../stores/auth'
import { useRouter } from 'vue-router'

const auth = useAuthStore()
const router = useRouter()

function logout() {
  auth.logout()
  router.push('/login')
}
</script>

<template>
  <div class="dashboard-layout">
    <aside class="sidebar">
      <div class="sidebar-header">⛏️ 矿山调度</div>
      <ul class="sidebar-nav">
        <li><router-link to="/">📊 运营看板</router-link></li>
        <li><router-link to="/drivers">🚛 司机管理</router-link></li>
        <li><router-link to="/pits">⛰️ 坑口管理</router-link></li>
        <li><router-link to="/waybills">📋 运单管理</router-link></li>
        <li><router-link to="/queue">🔢 队列监控</router-link></li>
        <li><router-link to="/alerts">🔔 告警中心</router-link></li>
      </ul>
      <div style="padding: 12px 20px; border-top: 1px solid var(--gray-700);">
        <div style="font-size: 12px; color: var(--gray-400); margin-bottom: 4px;">
          {{ auth.displayName }}
          <span style="color: var(--gray-500);">({{ auth.role }})</span>
        </div>
        <button class="btn btn-sm" style="color: var(--gray-400); padding: 4px 0;" @click="logout">
          退出登录
        </button>
      </div>
    </aside>

    <div class="main-area">
      <header class="topbar">
        <h1 class="topbar-title">矿山调度管理系统</h1>
        <div class="topbar-user">
          <span class="text-sm">{{ auth.displayName }}</span>
        </div>
      </header>

      <main class="page-content">
        <router-view />
      </main>
    </div>
  </div>
</template>
