<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { pitApi, queueApi, driverApi } from '../api/client'

interface Pit { id: string; name: string; current_queue_count: number }
interface QueueEntry { waybill_id: string; driver_id: string; queue_position: number; entered_at: string }
interface Driver { id: string; name: string; license_plate: string }

const pits = ref<Pit[]>([])
const queues = ref<Record<string, QueueEntry[]>>({})
const drivers = ref<Driver[]>([])
const selectedPit = ref<string>('')
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    const p = await pitApi.list()
    pits.value = p

    const d = await driverApi.list()
    drivers.value = d

    // Load queue for all pits in parallel
    const results = await Promise.allSettled(
      p.map(pit => queueApi.getPitQueue(pit.id))
    )
    const q: Record<string, QueueEntry[]> = {}
    p.forEach((pit, i) => {
      if (results[i].status === 'fulfilled') {
        q[pit.id] = (results[i] as PromiseFulfilledResult<QueueEntry[]>).value
      }
    })
    queues.value = q
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

function driverName(id: string) {
  return drivers.value.find(d => d.id === id)
}

onMounted(load)
</script>

<template>
  <div class="section-header">
    <h2>队列监控</h2>
    <div class="flex gap-2 items-center">
      <button class="btn btn-sm" style="background: var(--gray-200);" @click="load">🔄 刷新</button>
    </div>
  </div>

  <div v-if="loading" class="text-center" style="padding: 40px;">加载中...</div>

  <template v-else>
    <div class="stats-grid" style="grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));">
      <div v-for="pit in pits" :key="pit.id" class="card" style="cursor: pointer;" :style="selectedPit === pit.id ? 'border-color: var(--primary);' : ''" @click="selectedPit = selectedPit === pit.id ? '' : pit.id">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
          <strong>{{ pit.name }}</strong>
          <span class="badge" :class="(queues[pit.id]?.length || 0) > 0 ? 'badge-warning' : 'badge-success'">
            {{ queues[pit.id]?.length || 0 }} 辆排队
          </span>
        </div>
        <div v-if="queues[pit.id] && queues[pit.id]!.length > 0" class="text-sm">
          首辆: {{ driverName(queues[pit.id]![0].driver_id)?.name || '未知' }}
        </div>
        <div v-else class="text-sm" style="color: var(--gray-400);">队列为空</div>
      </div>
    </div>

    <div v-if="selectedPit && queues[selectedPit]" class="card">
      <h3 style="margin-bottom: 12px;">
        {{ pits.find(p => p.id === selectedPit)?.name }} - 实时队列
        <span class="badge badge-info" style="margin-left: 8px;">共 {{ queues[selectedPit]!.length }} 辆</span>
      </h3>
      <table>
        <thead>
          <tr>
            <th>序号</th>
            <th>司机</th>
            <th>车牌</th>
            <th>入队时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in queues[selectedPit]" :key="e.waybill_id">
            <td><strong>{{ e.queue_position }}</strong></td>
            <td>{{ driverName(e.driver_id)?.name || '未知' }}</td>
            <td>{{ driverName(e.driver_id)?.license_plate || '-' }}</td>
            <td class="text-sm">{{ new Date(e.entered_at).toLocaleString() }}</td>
          </tr>
          <tr v-if="queues[selectedPit]!.length === 0">
            <td colspan="4" class="text-center text-sm" style="padding: 24px;">队列为空</td>
          </tr>
        </tbody>
      </table>
    </div>
  </template>
</template>
