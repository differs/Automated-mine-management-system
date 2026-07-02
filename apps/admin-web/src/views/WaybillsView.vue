<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { waybillApi, driverApi, pitApi } from '../api/client'

interface Waybill {
  id: string; serial_no: string; driver_id: string; pit_id: string; status: string; dispatch_time: string | null
}
interface Driver { id: string; name: string }
interface Pit { id: string; name: string }

const waybills = ref<Waybill[]>([])
const drivers = ref<Driver[]>([])
const pits = ref<Pit[]>([])
const loading = ref(true)
const statusFilter = ref('')
const showForm = ref(false)
const form = ref({ driver_id: '', pit_id: '', estimated_weight_ton: 35 })

async function load() {
  loading.value = true
  try {
    const [w, d, p] = await Promise.all([
      waybillApi.list({ status: statusFilter.value || undefined }),
      driverApi.list(),
      pitApi.list(),
    ])
    waybills.value = w
    drivers.value = d
    pits.value = p
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function createWaybill() {
  try {
    await waybillApi.create(form.value)
    showForm.value = false
    await load()
  } catch (e) {
    alert(e instanceof Error ? e.message : '创建失败')
  }
}

async function dispatchWaybill(id: string) {
  try {
    await waybillApi.dispatch(id, '00000000-0000-0000-0000-000000000000')
    await load()
  } catch (e) {
    alert(e instanceof Error ? e.message : '派单失败')
  }
}

async function cancelWaybill(id: string) {
  const reason = prompt('请输入取消原因:')
  if (!reason) return
  try {
    await waybillApi.cancel(id, '00000000-0000-0000-0000-000000000000', reason)
    await load()
  } catch (e) {
    alert(e instanceof Error ? e.message : '取消失败')
  }
}

function driverName(id: string) {
  return drivers.value.find(d => d.id === id)?.name || id.slice(0, 8)
}

function pitName(id: string) {
  return pits.value.find(p => p.id === id)?.name || id.slice(0, 8)
}

const statusLabels: Record<string, string> = {
  pending_dispatch: '待派车',
  dispatched: '已派车',
  arrived: '已到场',
  queueing: '排队中',
  loading: '装载中',
  loaded: '已装载',
  weighing: '称重中',
  completed: '已完成',
  cancelled: '已取消',
}

const statusClasses: Record<string, string> = {
  pending_dispatch: 'badge-warning',
  dispatched: 'badge-info',
  arrived: 'badge-info',
  queueing: 'badge-info',
  loading: 'badge-warning',
  loaded: 'badge-info',
  weighing: 'badge-info',
  completed: 'badge-success',
  cancelled: 'badge-gray',
}

onMounted(load)
</script>

<template>
  <div class="section-header">
    <h2>运单管理</h2>
    <div class="flex gap-2 items-center">
      <select v-model="statusFilter" class="form-input" style="width: 120px;" @change="load">
        <option value="">全部状态</option>
        <option value="pending_dispatch">待派车</option>
        <option value="dispatched">已派车</option>
        <option value="completed">已完成</option>
        <option value="cancelled">已取消</option>
      </select>
      <button class="btn btn-primary" @click="showForm = true">+ 新建运单</button>
    </div>
  </div>

  <div v-if="showForm" class="card" style="margin-bottom: 16px;">
    <h3 style="margin-bottom: 12px;">新建运单</h3>
    <div class="flex gap-4" style="flex-wrap: wrap;">
      <div class="form-group" style="flex: 1; min-width: 200px;">
        <label>司机</label>
        <select v-model="form.driver_id" class="form-input">
          <option value="">请选择</option>
          <option v-for="d in drivers" :key="d.id" :value="d.id">{{ d.name }}</option>
        </select>
      </div>
      <div class="form-group" style="flex: 1; min-width: 200px;">
        <label>坑口</label>
        <select v-model="form.pit_id" class="form-input">
          <option value="">请选择</option>
          <option v-for="p in pits" :key="p.id" :value="p.id">{{ p.name }}</option>
        </select>
      </div>
      <div class="form-group" style="flex: 1; min-width: 120px;">
        <label>预估重量(吨)</label>
        <input v-model.number="form.estimated_weight_ton" class="form-input" type="number" />
      </div>
    </div>
    <div class="flex gap-2" style="margin-top: 12px;">
      <button class="btn btn-primary" @click="createWaybill">创建运单</button>
      <button class="btn" style="background: var(--gray-200);" @click="showForm = false">取消</button>
    </div>
  </div>

  <div class="card">
    <table>
      <thead>
        <tr>
          <th>运单号</th>
          <th>司机</th>
          <th>坑口</th>
          <th>状态</th>
          <th>派单时间</th>
          <th>操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="w in waybills" :key="w.id">
          <td><strong>{{ w.serial_no }}</strong></td>
          <td>{{ driverName(w.driver_id) }}</td>
          <td>{{ pitName(w.pit_id) }}</td>
          <td><span class="badge" :class="statusClasses[w.status] || 'badge-gray'">{{ statusLabels[w.status] || w.status }}</span></td>
          <td class="text-sm">{{ w.dispatch_time ? new Date(w.dispatch_time).toLocaleString() : '-' }}</td>
          <td>
            <div class="flex gap-2">
              <button v-if="w.status === 'pending_dispatch'" class="btn btn-sm btn-success" @click="dispatchWaybill(w.id)">派单</button>
              <button v-if="!['completed', 'cancelled'].includes(w.status)" class="btn btn-sm btn-danger" @click="cancelWaybill(w.id)">取消</button>
              <span v-if="w.status === 'completed'" class="text-sm">✅ 完成</span>
              <span v-if="w.status === 'cancelled'" class="text-sm">已取消</span>
            </div>
          </td>
        </tr>
        <tr v-if="waybills.length === 0">
          <td colspan="6" class="text-center text-sm" style="padding: 24px;">暂无运单数据</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
