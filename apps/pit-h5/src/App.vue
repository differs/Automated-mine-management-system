<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'

const API = '/api/v1'

interface Pit { id: string; name: string; current_queue_count: number; is_active: boolean }
interface Driver { id: string; name: string; license_plate: string }
interface QueueEntry { waybill_id: string; driver_id: string; queue_position: number; entered_at: string }
interface Waybill { id: string; serial_no: string; driver_id: string; pit_id: string; status: string; estimated_weight_ton: number | null }

const operatorName = ref('')
const selectedPitId = ref('')
const loggedIn = ref(false)
const pits = ref<Pit[]>([])
const drivers = ref<Driver[]>([])
const queue = ref<QueueEntry[]>([])
const loadingList = ref(false)
const currentLoading = ref<Waybill | null>(null)
const tab = ref<'queue' | 'loading'>('queue')
const finishSuffix = ref('')
const error = ref('')
let pollTimer: ReturnType<typeof setInterval> | null = null

const driverMap = computed(() => {
  const m: Record<string, Driver> = {}
  drivers.value.forEach(d => { m[d.id] = d })
  return m
})

async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  const res = await fetch(`${API}${path}`, { ...options, headers })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }))
    throw new Error(err.message || '请求失败')
  }
  return res.json()
}

async function loadData() {
  try {
    const [p, d] = await Promise.all([api<Pit[]>('/pits'), api<Driver[]>('/drivers')])
    pits.value = p; drivers.value = d
  } catch (e) { console.error(e) }
}

async function enterPit() {
  if (!selectedPitId.value || !operatorName.value.trim()) { error.value = '请选择坑口并输入操作员名称'; return }
  loggedIn.value = true; error.value = ''
  await refreshQueue()
  startPolling()
}

async function refreshQueue() {
  if (!selectedPitId.value) return
  loadingList.value = true
  try {
    const [q, waybills] = await Promise.all([
      api<QueueEntry[]>(`/queue/pits/${selectedPitId.value}`),
      api<Waybill[]>(`/waybills?pit_id=${selectedPitId.value}`)
    ])
    queue.value = q
    currentLoading.value = waybills.find(w => w.status === 'loading') || null
  } catch (e) { console.error(e) }
  finally { loadingList.value = false }
}

async function callNext(waybillId: string) {
  try {
    const wb = await api<Waybill>(`/waybills/${waybillId}`)
    if (wb.status === 'queueing') {
      await api(`/queue/waybills/${waybillId}/call-next`, {
        method: 'POST', body: JSON.stringify({ operator_id: operatorName.value })
      })
    }
    await api(`/loading/waybills/${waybillId}/start`, {
      method: 'POST', body: JSON.stringify({ operator_id: operatorName.value })
    })
    await refreshQueue()
  } catch (e: any) { alert('操作失败: ' + e.message) }
}

async function finishLoading() {
  if (!finishSuffix.value.trim()) { alert('请输入运单号后4位'); return }
  try {
    const waybills = await api<Waybill[]>(`/waybills?pit_id=${selectedPitId.value}&status=loading`)
    const target = waybills.find(w => w.serial_no.endsWith(finishSuffix.value.trim()))
    if (!target) { alert('未找到匹配的装载中运单'); return }

    await api(`/loading/waybills/${target.id}/finish`, {
      method: 'POST', body: JSON.stringify({ operator_id: operatorName.value })
    })
    // auto weigh with estimated weight
    await api(`/weighing/waybills/${target.id}`, {
      method: 'POST', body: JSON.stringify({
        operator_id: operatorName.value,
        gross_weight_ton: target.estimated_weight_ton || 30,
        tare_weight_ton: 0,
        net_weight_ton: target.estimated_weight_ton || 30,
        source: 'manual'
      })
    })
    finishSuffix.value = ''
    await refreshQueue()
  } catch (e: any) { alert('操作失败: ' + e.message) }
}

function startPolling() { pollTimer = setInterval(refreshQueue, 15000) }
function logout() { loggedIn.value = false; operatorName.value = ''; selectedPitId.value = ''; if (pollTimer) clearInterval(pollTimer) }

onMounted(loadData)
onUnmounted(() => { if (pollTimer) clearInterval(pollTimer) })
</script>

<template>
  <div class="page">
    <!-- Header -->
    <div class="header">
      <div class="header-left">
        <h1>⛰️ 坑口作业端</h1>
        <p v-if="loggedIn" class="header-sub">{{ pits.find(p => p.id === selectedPitId)?.name }}</p>
      </div>
      <div v-if="loggedIn" class="header-right">
        <span class="op-name">{{ operatorName }}</span>
        <button class="btn-xs" @click="logout">退出</button>
      </div>
    </div>

    <!-- Login -->
    <div v-if="!loggedIn" class="section">
      <div class="card">
        <div class="card-icon">⛰️</div>
        <h2>坑口登录</h2>
        <select v-model="selectedPitId" class="input">
          <option value="">-- 选择坑口 --</option>
          <option v-for="p in pits" :key="p.id" :value="p.id">
            {{ p.name }} (排队: {{ p.current_queue_count }})
          </option>
        </select>
        <input v-model="operatorName" class="input" placeholder="操作员名称" />
        <div v-if="error" class="error-msg">{{ error }}</div>
        <button class="btn btn-primary btn-full" @click="enterPit">进入坑口</button>
      </div>
    </div>

    <!-- Operations -->
    <div v-else>
      <!-- Tab bar -->
      <div class="tabs">
        <button :class="['tab', { active: tab === 'queue' }]" @click="tab = 'queue'">🔢 队列 ({{ queue.length }})</button>
        <button :class="['tab', { active: tab === 'loading' }]" @click="tab = 'loading'">🚜 装车中</button>
      </div>

      <!-- Queue Tab -->
      <div v-if="tab === 'queue'" class="section">
        <div v-if="currentLoading" class="loading-banner">
          🚜 正在装车: {{ driverMap[currentLoading.driver_id]?.name || '未知' }} ({{ driverMap[currentLoading.driver_id]?.license_plate || '' }})
        </div>

        <div v-if="queue.length === 0" class="card empty-card">
          <div class="empty-icon">🟢</div>
          <h3>队列为空</h3>
          <p class="text-muted">等待司机到场排队</p>
        </div>

        <div v-else class="queue-list">
          <div v-for="entry in queue" :key="entry.waybill_id" class="queue-item">
            <div class="queue-pos">{{ entry.queue_position }}</div>
            <div class="queue-info">
              <div class="driver-name">{{ driverMap[entry.driver_id]?.name || '未知司机' }}</div>
              <div class="driver-plate">{{ driverMap[entry.driver_id]?.license_plate || '' }}</div>
              <div class="queue-time">{{ new Date(entry.entered_at).toLocaleTimeString() }} 入队</div>
            </div>
            <button class="btn-call" @click="callNext(entry.waybill_id)" :disabled="!!currentLoading">
              {{ currentLoading ? '装车中' : '叫号' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Loading Tab -->
      <div v-if="tab === 'loading'" class="section">
        <div class="card">
          <h3>完成装车</h3>
          <p class="text-muted" style="margin-bottom:12px;">输入当前装车辆运单号后4位</p>
          <div class="finish-row">
            <input v-model="finishSuffix" class="input" style="flex:1;margin:0;" placeholder="运单号后4位" @keyup.enter="finishLoading" />
            <button class="btn btn-success" @click="finishLoading">完成</button>
          </div>
        </div>

        <div v-if="currentLoading" class="card">
          <h3>当前装车</h3>
          <div class="info-row"><span class="info-label">司机</span><span class="info-value">{{ driverMap[currentLoading.driver_id]?.name || '未知' }}</span></div>
          <div class="info-row"><span class="info-label">车牌</span><span class="info-value">{{ driverMap[currentLoading.driver_id]?.license_plate || '未知' }}</span></div>
          <div class="info-row"><span class="info-label">运单号</span><span class="info-value" style="font-size:12px;">{{ currentLoading.serial_no }}</span></div>
        </div>
        <div v-else class="card empty-card" style="padding:24px;">
          <div class="empty-icon">📭</div>
          <p class="text-muted">暂无装车任务</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'PingFang SC', sans-serif;
  font-size: 14px; background: #f0f2f5; color: #333;
}
.page { max-width: 600px; margin: 0 auto; background: #f0f2f5; min-height: 100vh; padding-bottom: 24px; }

.header {
  background: linear-gradient(135deg, #065f46, #16a34a);
  color: white; padding: 16px; display: flex; justify-content: space-between; align-items: center;
  position: sticky; top: 0; z-index: 10;
}
.header-left h1 { font-size: 18px; }
.header-sub { font-size: 12px; opacity: 0.8; }
.header-right { display: flex; align-items: center; gap: 8px; }
.op-name { font-size: 12px; opacity: 0.8; }
.btn-xs { padding: 4px 10px; font-size: 12px; background: rgba(255,255,255,0.15); color: white; border: none; border-radius: 6px; cursor: pointer; }

.section { padding: 12px 16px; }

.card {
  background: white; border-radius: 14px; padding: 24px; margin-bottom: 12px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06); text-align: center;
}
.card-icon { font-size: 40px; margin-bottom: 12px; }
.card h2 { font-size: 20px; margin-bottom: 4px; }
.card h3 { font-size: 16px; margin-bottom: 12px; text-align: left; }

.input {
  width: 100%; padding: 12px 16px; border: 1.5px solid #d1d5db; border-radius: 10px;
  font-size: 15px; margin-bottom: 10px; outline: none;
}
.input:focus { border-color: #16a34a; box-shadow: 0 0 0 3px rgba(22,163,74,0.1); }

.btn {
  padding: 10px 20px; border: none; border-radius: 10px; font-size: 14px; font-weight: 600;
  cursor: pointer; transition: all 0.15s;
}
.btn-full { width: 100%; justify-content: center; }
.btn-primary { background: #2563eb; color: white; }
.btn-success { background: #16a34a; color: white; }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }

.error-msg { color: #dc2626; font-size: 13px; margin-bottom: 8px; }
.text-muted { color: #9ca3af; font-size: 13px; }

/* Tabs */
.tabs { display: flex; background: white; border-bottom: 1px solid #e5e7eb; }
.tab { flex: 1; padding: 12px; text-align: center; font-size: 14px; font-weight: 500; border: none; background: transparent; cursor: pointer; border-bottom: 2px solid transparent; }
.tab.active { color: #16a34a; border-bottom-color: #16a34a; font-weight: 600; }

/* Loading banner */
.loading-banner {
  background: #dbeafe; border-radius: 10px; padding: 12px 16px; margin-bottom: 12px;
  font-size: 13px; font-weight: 500; display: flex; align-items: center; gap: 8px;
}

/* Queue */
.queue-list { display: flex; flex-direction: column; gap: 8px; }
.queue-item {
  background: white; border-radius: 12px; padding: 12px 16px;
  display: flex; align-items: center; gap: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.04);
}
.queue-pos {
  width: 36px; height: 36px; border-radius: 50%; background: #16a34a; color: white;
  display: flex; align-items: center; justify-content: center; font-weight: 700; font-size: 15px; flex-shrink: 0;
}
.queue-info { flex: 1; }
.driver-name { font-weight: 600; font-size: 15px; }
.driver-plate { font-size: 12px; color: #6b7280; }
.queue-time { font-size: 11px; color: #9ca3af; }
.btn-call {
  padding: 8px 16px; background: #f59e0b; color: white; border: none; border-radius: 8px;
  font-size: 13px; font-weight: 600; cursor: pointer; white-space: nowrap;
}

/* Finish row */
.finish-row { display: flex; gap: 8px; align-items: center; }

/* Empty */
.empty-card { padding: 32px 24px; }
.empty-icon { font-size: 40px; margin-bottom: 8px; }

.info-row { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #f3f4f6; }
.info-row:last-child { border-bottom: none; }
.info-label { color: #6b7280; font-size: 13px; }
.info-value { font-weight: 500; }
</style>
