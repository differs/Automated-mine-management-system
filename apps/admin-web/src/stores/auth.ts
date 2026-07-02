import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi, setToken } from '../api/client'

export const useAuthStore = defineStore('auth', () => {
  const token = ref(localStorage.getItem('access_token') || '')
  const refreshToken = ref(localStorage.getItem('refresh_token') || '')
  const role = ref(localStorage.getItem('role') || '')
  const displayName = ref(localStorage.getItem('display_name') || '')

  const isLoggedIn = computed(() => !!token.value)

  async function login(username: string, password: string) {
    const res = await authApi.login(username, password)
    token.value = res.access_token
    refreshToken.value = res.refresh_token
    role.value = res.role
    displayName.value = res.display_name
    setToken(res.access_token)
    localStorage.setItem('access_token', res.access_token)
    localStorage.setItem('refresh_token', res.refresh_token)
    localStorage.setItem('role', res.role)
    localStorage.setItem('display_name', res.display_name)
  }

  function logout() {
    token.value = ''
    refreshToken.value = ''
    role.value = ''
    displayName.value = ''
    setToken(null)
    localStorage.removeItem('access_token')
    localStorage.removeItem('refresh_token')
    localStorage.removeItem('role')
    localStorage.removeItem('display_name')
  }

  // Restore token from localStorage on init
  if (token.value) {
    setToken(token.value)
  }

  return { token, refreshToken, role, displayName, isLoggedIn, login, logout }
})
