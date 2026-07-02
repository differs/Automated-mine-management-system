<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const auth = useAuthStore()
const router = useRouter()
const username = ref('')
const password = ref('')
const error = ref('')
const loading = ref(false)

async function handleLogin() {
  if (!username.value || !password.value) {
    error.value = '请输入用户名和密码'
    return
  }
  loading.value = true
  error.value = ''
  try {
    await auth.login(username.value, password.value)
    router.push('/')
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : '登录失败'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <h1>矿山调度管理系统</h1>
      <p>Automated Mine Management System</p>

      <form @submit.prevent="handleLogin">
        <div class="form-group">
          <label>用户名</label>
          <input
            v-model="username"
            class="form-input"
            type="text"
            placeholder="请输入用户名"
            autocomplete="username"
          />
        </div>
        <div class="form-group">
          <label>密码</label>
          <input
            v-model="password"
            class="form-input"
            type="password"
            placeholder="请输入密码"
            autocomplete="current-password"
          />
        </div>

        <div v-if="error" style="color: var(--danger); margin-bottom: 12px; font-size: 13px;">
          {{ error }}
        </div>

        <button type="submit" class="btn btn-primary" style="width: 100%; justify-content: center;" :disabled="loading">
          {{ loading ? '登录中...' : '登录' }}
        </button>

        <div style="margin-top: 16px; font-size: 12px; color: var(--gray-400); text-align: center;">
          演示账号: admin / admin123
        </div>
      </form>
    </div>
  </div>
</template>
