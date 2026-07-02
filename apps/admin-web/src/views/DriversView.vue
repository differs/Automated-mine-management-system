<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { driverApi } from '../api/client'

interface Driver {
  id: string; name: string; phone: string; license_plate: string; vehicle_type: string; status: string
}

const drivers = ref<Driver[]>([])
const loading = ref(true)
const keyword = ref('')
const statusFilter = ref('')
const showForm = ref(false)
const form = ref({ name: '', phone: '', license_plate: '', vehicle_type: 'dump_truck', capacity_ton: 30 })

async function load() {
  loading.value = true
  try {
    drivers.value = await driverApi.list({
      keyword: keyword.value || undefined,
      status: statusFilter.value || undefined,
    })
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function createDriver() {
  try {
    await driverApi.create(form.value)
    showForm.value = false
    form.value = { name: '', phone: '', license_plate: '', vehicle_type: 'dump_truck', capacity_ton: 30 }
    await load()
  } catch (e) {
    alert(e instanceof Error ? e.message : '创建失败')
  }
}

onMounted(load)
</script>

<template>
  <div class="section-header">
    <h2>司机管理</h2>
    <div class="flex gap-2 items-center">
      <input v-model="keyword" class="form-input" style="width: 200px;" placeholder="搜索姓名/电话/车牌" @input="load" />
      <select v-model="statusFilter" class="form-input" style="width: 120px;" @change="load">
        <option value="">全部状态</option>
        <option value="idle">空闲</option>
        <option value="working">工作中</option>
        <option value="offline">离线</option>
      </select>
      <button class="btn btn-primary" @click="showForm = true">+ 新增司机</button>
    </div>
  </div>

  <div v-if="showForm" class="card" style="margin-bottom: 16px;">
    <h3 style="margin-bottom: 12px;">新增司机</h3>
    <div class="flex gap-4" style="flex-wrap: wrap;">
      <div class="form-group" style="flex: 1; min-width: 150px;">
        <label>姓名</label>
        <input v-model="form.name" class="form-input" />
      </div>
      <div class="form-group" style="flex: 1; min-width: 150px;">
        <label>手机号</label>
        <input v-model="form.phone" class="form-input" />
      </div>
      <div class="form-group" style="flex: 1; min-width: 150px;">
        <label>车牌号</label>
        <input v-model="form.license_plate" class="form-input" />
      </div>
      <div class="form-group" style="flex: 1; min-width: 120px;">
        <label>车型</label>
        <select v-model="form.vehicle_type" class="form-input">
          <option value="dump_truck">自卸车</option>
          <option value="trailer">挂车</option>
          <option value="other">其他</option>
        </select>
      </div>
      <div class="form-group" style="flex: 1; min-width: 120px;">
        <label>载重(吨)</label>
        <input v-model.number="form.capacity_ton" class="form-input" type="number" />
      </div>
    </div>
    <div class="flex gap-2" style="margin-top: 12px;">
      <button class="btn btn-primary" @click="createDriver">保存</button>
      <button class="btn" style="background: var(--gray-200);" @click="showForm = false">取消</button>
    </div>
  </div>

  <div class="card">
    <table>
      <thead>
        <tr>
          <th>姓名</th>
          <th>手机号</th>
          <th>车牌号</th>
          <th>车型</th>
          <th>状态</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in drivers" :key="d.id">
          <td><strong>{{ d.name }}</strong></td>
          <td>{{ d.phone }}</td>
          <td>{{ d.license_plate }}</td>
          <td>{{ d.vehicle_type === 'dump_truck' ? '自卸车' : d.vehicle_type === 'trailer' ? '挂车' : '其他' }}</td>
          <td>
            <span class="badge" :class="d.status === 'idle' ? 'badge-success' : d.status === 'working' ? 'badge-info' : 'badge-gray'">
              {{ d.status === 'idle' ? '空闲' : d.status === 'working' ? '工作中' : '离线' }}
            </span>
          </td>
        </tr>
        <tr v-if="drivers.length === 0">
          <td colspan="5" class="text-center text-sm" style="padding: 24px;">暂无司机数据</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
