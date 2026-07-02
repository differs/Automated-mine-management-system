<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

const API = '/api/v1'

// State
const phone = ref('')
const loggedIn = ref(false)
const driverName = ref('')
const driverId = ref('')
const loading = ref(false)
const error = ref('')

interface Waybill {
  id: string; serial_no: string; driver_id: string; pit_id: string
  status: string; dispatch_time: string | null; estimated_weight_ton: number | null
}

interface Pit { id: string; name: string }
interface Driver { id: string; name: string; phone: string; license_plate: string }

const currentTask = ref<Waybill | null>(null)
const noTask = ref(true)
const pitNames = ref<Record<string, string>>({})
let pollTimer: ReturnType<typeof setInterval> | null = null

const statusLabels: Record<string, string> = {
  pending_dispatch: '待派车', dispatched: '已派车',
  arrived: '已到场', queueing: '排队中',
  loading: '装载中', loaded: '已装载',
  weighing: '称重中', completed: '已完成', cancelled: '已取消',
}

async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  const res = await fetch(`${API}${path}`, { ...options, headers })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }))
    throw new Error(err.message || '请求失败')
  }
  return res.json()
}

async function doLogin() {
  if (!phone.value.trim()) { error.value = '请输入手机号'; return }
  loading.value = true; error.value = ''
  try {
    const drivers = await api<Driver[]>(`/drivers?keyword=${encodeURIComponent(phone.value)}`)
    let driver = drivers.find(d => d.phone === phone.value)
    if (!driver) {
      driver = await api<Driver>('/drivers', {
        method: 'POST',
        body: JSON.stringify({
          name: `司机${phone.value.slice(-4)}`, phone: phone.value,
          license_plate: `临时${phone.value.slice(-4)}`,
          vehicle_type: 'dump_truck', capacity_ton: 30
        })
      })
    }
    driverId.value = driver.id; driverName.value = driver.name
    loggedIn.value = true
    await loadTask()
    startPolling()
  } catch (e: any) { error.value = e.message } finally { loading.value = false }
}

async function loadTask() {
  try {
    const waybills = await api<Waybill[]>(`/waybills`, {
      headers: { 'Content-Type': 'application/json', 'X-Driver-Id': driverId.value }
    })
    // Filter waybills for this driver
    const mine = waybills.filter(w => w.driver_id === driverId.value)
    const active = mine.find(w => !['completed', 'cancelled'].includes(w.status))
    currentTask.value = active || null
    noTask.value = !active

    // Load pit names
    const pits = await api<Pit[]>('/pits')
    const pmap: Record<string, string> = {}
    pits.forEach(p => { pmap[p.id] = p.name })
    pitNames.value = pmap
  } catch (e) { console.error('load task error:', e) }
}

async function arrive() {
  if (!currentTask.value) return
  try {
    await api(`/waybills/${currentTask.value.id}/arrive`, {
      method: 'POST', body: JSON.stringify({ arrival_source: 'driver_app' })
    })
    alert('✅ 签到成功！请等待叫号')
    await loadTask()
  } catch (e: any) { alert('签到失败: ' + e.message) }
}

async function cancelTask() {
  if (!currentTask.value) return
  const reason = prompt('请输入取消原因:')
  if (!reason) return
  try {
    await api(`/waybills/${currentTask.value.id}/cancel`, {
      method: 'POST',
      body: JSON.stringify({ cancelled_by: driverId.value, reason })
    })
    alert('✅ 任务已取消')
    await loadTask()
  } catch (e: any) { alert('取消失败: ' + e.message) }
}

function startPolling() { pollTimer = setInterval(loadTask, 15000) }
function logout() { loggedIn.value = false; phone.value = ''; currentTask.value = null; driverName.value = ''; if (pollTimer) clearInterval(pollTimer) }

onUnmounted(() => { if (pollTimer) clearInterval(pollTimer) })
</script>

<template>
  <div class="page">
    <!-- Header -->
    <div class="header">
      <div class="header-left">
        <h1>🚛 司机端</h1>
        <p v-if="loggedIn" class="header-name">{{ driverName }}</p>
      </div>
      <div v-if="loggedIn" class="header-right">
        <button class="btn btn-xs" @click="logout">退出</button>
      </div>
    </div>

    <!-- Login -->
    <div v-if="!loggedIn" class="section">
      <div class="card">
        <div class="card-icon">🚛</div>
        <h2>司机登录</h2>
        <p class="text-muted">输入手机号登录 / 自动注册</p>
        <input v-model="phone" class="input" type="tel" placeholder="输入手机号" @keyup.enter="doLogin" />
        <div v-if="error" class="error-msg">{{ error }}</div>
        <button class="btn btn-primary btn-full" :disabled="loading" @click="doLogin">
          {{ loading ? '登录中...' : '登录' }}
        </button>
      </div>
    </div>

    <!-- Main -->
    <div v-else class="section">
      <!-- Current Task -->
      <div v-if="currentTask" class="card task-card">
        <div class="task-header">
          <h3>当前任务</h3>
          <span class="badge" :class="currentTask.status">{{ statusLabels[currentTask.status] || currentTask.status }}</span>
        </div>
        <div class="task-body">
          <div class="info-row">
            <span class="info-label">运单号</span>
            <span class="info-value">{{ currentTask.serial_no }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">坑口</span>
            <span class="info-value">{{ pitNames[currentTask.pit_id] || currentTask.pit_id.slice(0, 8) }}</span>
          </div>
          <div class="info-row">
            <span class="info-label">预估重量</span>
            <span class="info-value">{{ currentTask.estimated_weight_ton || '-' }} 吨</span>
          </div>
        </div>

        <div class="task-actions">
          <button v-if="currentTask.status === 'dispatched'" class="btn btn-success btn-full" @click="arrive">
            ✅ 我已到场
          </button>
          <div v-if="currentTask.status === 'queueing'" class="queue-info">
            <span class="queue-dot pulse"></span>
            <span>排队中，请等待叫号...</span>
          </div>
          <div v-if="currentTask.status === 'loading'" class="queue-info loading">
            <span>🚜 正在装车中...</span>
          </div>
          <div v-if="currentTask.status === 'loaded' || currentTask.status === 'weighing'" class="queue-info">
            <span>⚖️ 请前往地磅称重</span>
          </div>
          <button v-if="!['completed', 'cancelled'].includes(currentTask.status)" class="btn btn-outline btn-full" @click="cancelTask">
            放弃任务
          </button>
        </div>
      </div>

      <!-- No Task -->
      <div v-else class="card empty-card">
        <div class="empty-icon">📭</div>
        <h3>暂无任务</h3>
        <p class="text-muted">等待调度员派单...</p>
        <div class="loading-dots">
          <span class="dot"></span><span class="dot"></span><span class="dot"></span>
        </div>
      </div>

      <!-- Tips -->
      <div class="tips-card">
        <div class="tip-item">💡 系统每15秒自动刷新任务状态</div>
        <div class="tip-item">📞 如有问题请联系调度员</div>
      </div>
    </div>
  </div>
</template>

<style>
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'PingFang SC', sans-serif;
  font-size: 14px; background: #f0f2f5; color: #333; -webkit-font-smoothing: antialiased;
}
.page { max-width: 480px; margin: 0 auto; background: #f0f2f5; min-height: 100vh; padding-bottom: 24px; }

.header {
  background: linear-gradient(135deg, #1e3a5f, #2563eb);
  color: white; padding: 20px 16px; display: flex; justify-content: space-between; align-items: center;
  position: sticky; top: 0; z-index: 10;
}
.header-left h1 { font-size: 18px; }
.header-name { font-size: 12px; opacity: 0.8; margin-top: 2px; }
.header-right { display: flex; gap: 8px; }

.section { padding: 16px; }

.card {
  background: white; border-radius: 14px; padding: 24px; margin-bottom: 12px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06); text-align: center;
}
.card-icon { font-size: 40px; margin-bottom: 12px; }
.card h2 { font-size: 20px; margin-bottom: 4px; }
.card h3 { font-size: 16px; margin-bottom: 8px; }

.input {
  width: 100%; padding: 12px 16px; border: 1.5px solid #d1d5db; border-radius: 10px;
  font-size: 16px; margin: 16px 0 12px; outline: none; transition: border-color 0.2s;
}
.input:focus { border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,0.1); }

.btn {
  padding: 10px 20px; border: none; border-radius: 10px; font-size: 15px; font-weight: 600;
  cursor: pointer; transition: all 0.15s; display: inline-flex; align-items: center; gap: 6px;
}
.btn-full { width: 100%; justify-content: center; }
.btn-primary { background: #2563eb; color: white; }
.btn-primary:hover { background: #1d4ed8; }
.btn-primary:disabled { background: #93b4f5; }
.btn-success { background: #16a34a; color: white; }
.btn-success:hover { background: #15803d; }
.btn-outline { background: white; color: #2563eb; border: 1.5px solid #2563eb; margin-top: 8px; }
.btn-xs { padding: 4px 12px; font-size: 12px; background: rgba(255,255,255,0.15); color: white; border-radius: 6px; border: none; cursor: pointer; }

.error-msg { color: #dc2626; font-size: 13px; margin-bottom: 8px; }
.text-muted { color: #9ca3af; font-size: 13px; }

/* Task card */
.task-card { text-align: left; }
.task-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.task-body { margin-bottom: 16px; }

.badge {
  display: inline-block; padding: 3px 10px; border-radius: 999px; font-size: 11px; font-weight: 600;
}
.badge.dispatched { background: #dbeafe; color: #1e40af; }
.badge.queueing { background: #fef3c7; color: #92400e; }
.badge.loading { background: #dbeafe; color: #1e40af; }
.badge.arrived { background: #dbeafe; color: #1e40af; }
.badge.completed { background: #dcfce7; color: #166534; }
.badge.cancelled { background: #f3f4f6; color: #6b7280; }

.info-row { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #f3f4f6; }
.info-row:last-child { border-bottom: none; }
.info-label { color: #6b7280; font-size: 13px; }
.info-value { font-weight: 500; font-size: 14px; }

.task-actions { margin-top: 4px; }

.queue-info {
  background: #fef3c7; border-radius: 10px; padding: 12px 16px; margin-bottom: 8px;
  display: flex; align-items: center; gap: 8px; font-size: 14px; font-weight: 500;
}
.queue-info.loading { background: #dbeafe; }
.queue-dot { width: 8px; height: 8px; background: #d97706; border-radius: 50%; display: inline-block; }
.pulse { animation: pulse 1.5s infinite; }
@keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.3; } 100% { opacity: 1; } }

/* Empty */
.empty-card { padding: 48px 24px; }
.empty-icon { font-size: 48px; margin-bottom: 12px; }
.loading-dots { display: flex; gap: 6px; justify-content: center; margin-top: 16px; }
.loading-dots .dot {
  width: 8px; height: 8px; background: #d1d5db; border-radius: 50%;
  animation: bounce 1.4s infinite ease-in-out both;
}
.loading-dots .dot:nth-child(1) { animation-delay: -0.32s; }
.loading-dots .dot:nth-child(2) { animation-delay: -0.16s; }
@keyframes bounce { 0%, 80%, 100% { transform: scale(0); } 40% { transform: scale(1); } }

/* Tips */
.tips-card { background: white; border-radius: 10px; padding: 12px 16px; font-size: 12px; color: #6b7280; }
.tip-item { padding: 4px 0; }
</style>
