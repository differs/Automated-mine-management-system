<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { alertApi } from '../api/client'

interface Alert {
  id: string; waybill_id: string; type: string; severity: number; description: string; status: string; created_at: string; resolved_at: string | null
}

const alerts = ref<Alert[]>([])
const loading = ref(true)
const statusFilter = ref('open')
const typeFilter = ref('')

const alertTypeLabels: Record<string, string> = {
  late_arrival: '超时未到',
  queue_jump: '插队',
  loading_timeout: '装车超时',
  weight_deviation: '重量偏差',
  left_without_weighing: '离场未称重',
  manual_override: '手工干预',
  other: '其他',
}

async function load() {
  loading.value = true
  try {
    alerts.value = await alertApi.list({
      status: statusFilter.value || undefined,
      type: typeFilter.value || undefined,
    })
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function resolveAlert(id: string) {
  try {
    await alertApi.resolve(id)
    await load()
  } catch (e) {
    alert(e instanceof Error ? e.message : '操作失败')
  }
}

onMounted(load)
</script>

<template>
  <div class="section-header">
    <h2>告警中心</h2>
    <div class="flex gap-2 items-center">
      <select v-model="statusFilter" class="form-input" style="width: 120px;" @change="load">
        <option value="open">待处理</option>
        <option value="resolved">已解决</option>
        <option value="">全部</option>
      </select>
      <select v-model="typeFilter" class="form-input" style="width: 140px;" @change="load">
        <option value="">全部类型</option>
        <option v-for="(label, key) in alertTypeLabels" :key="key" :value="key">{{ label }}</option>
      </select>
      <button class="btn btn-sm" style="background: var(--gray-200);" @click="load">🔄 刷新</button>
    </div>
  </div>

  <div class="card">
    <table>
      <thead>
        <tr>
          <th>类型</th>
          <th>描述</th>
          <th>严重程度</th>
          <th>状态</th>
          <th>时间</th>
          <th>操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="a in alerts" :key="a.id">
          <td>
            <span class="badge" :class="a.type === 'weight_deviation' ? 'badge-danger' : 'badge-warning'">
              {{ alertTypeLabels[a.type] || a.type }}
            </span>
          </td>
          <td>{{ a.description }}</td>
          <td>
            <span class="badge" :class="a.severity > 2 ? 'badge-danger' : 'badge-warning'">
              {{ a.severity }}
            </span>
          </td>
          <td>
            <span class="badge" :class="a.status === 'open' ? 'badge-danger' : 'badge-success'">
              {{ a.status === 'open' ? '待处理' : '已解决' }}
            </span>
          </td>
          <td class="text-sm">{{ new Date(a.created_at).toLocaleString() }}</td>
          <td>
            <button v-if="a.status === 'open'" class="btn btn-sm btn-success" @click="resolveAlert(a.id)">标记已处理</button>
            <span v-else class="text-sm">{{ a.resolved_at ? new Date(a.resolved_at).toLocaleString() : '-' }}</span>
          </td>
        </tr>
        <tr v-if="alerts.length === 0">
          <td colspan="6" class="text-center text-sm" style="padding: 24px;">暂无告警数据 ✅</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
