<script setup lang="ts">
import { useAuthStore } from './stores/auth'
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'

const auth = useAuthStore()
const router = useRouter()

onMounted(() => {
  if (!auth.isLoggedIn && router.currentRoute.value.path !== '/login') {
    router.push('/login')
  }
})
</script>

<template>
  <div class="app-shell">
    <router-view />
  </div>
</template>
