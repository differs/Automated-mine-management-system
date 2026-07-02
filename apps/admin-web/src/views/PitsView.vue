<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { pitApi } from '../api/client'

interface Pit {
  id: string; name: string; code: string; current_queue_count: number; avg_wait_minutes: number; is_active: boolean
}

const pits = ref<Pit[]>([])
const loading = ref(true)
const showForm = ref(false)
const form = ref({ name: '', code: '', location_text: '', queue_capacity: 15 })

async function load() {
  loading.value = true
  try {
    pits.value = await pitApi.list()
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function createPit() {
  try {
    await pitApi.create(form.value)
    showForm.value = false
    form.value = { name: '', code: '', location_text: '', queue_capacity: 15 }
    await load()
  } catch (e) {
    alert(e instanceof Error ? e.message : '创建失败')
  }
}

onMounted(load)
</script>

<template>
  <div class="section-header">
    <h2>坑口管理</h2>
    <button class="btn btn-primary" @click="showForm = true">+ 新增坑口</button>
  </div>

  <div v-if="showForm" class="card" style="margin-bottom: 16px;">
    <h3 style="margin-bottom: 12px;">新增坑口</h3>
    <div class="flex gap-4" style="flex-wrap: wrap;">
      <div class="form-group" style="flex: 1; min-width: 150px;">
        <label>坑口名称</label>
        <input v-model="form.name" class="form-input" />
      </div>
      <div class="form-group" style="flex: 1; min-width: 120px;">
        <label>编号</label>
        <input v-model="form.code" class="form-input" />
      </div>
      <div class="form-group" style="flex: 1; min-width: 150px;">
        <label>位置</label>
        <input v-model="form.location_text" class="form-input" />
      </div>
      <div class="form-group" style="flex: 1; min-width: 120px;">
        <label>排队容量</label>
        <input v-model.number="form.queue_capacity" class="form-input" type="number" />
      </div>
    </div>
    <div class="flex gap-2" style="margin-top: 12px;">
      <button class="btn btn-primary" @click="createPit">保存</button>
      <button class="btn" style="background: var(--gray-200);" @click="showForm = false">取消</button>
    </div>
  </div>

  <div class="card">
    <table>
      <thead>
        <tr>
          <th>坑口名称</th>
          <th>编号</th>
          <th>当前排队</th>
          <th>平均等待(分钟)</th>
          <th>状态</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="p in pits" :key="p.id">
          <td><strong>{{ p.name }}</strong></td>
          <td>{{ p.code }}</td>
          <td>
            <span class="badge" :class="p.current_queue_count > 5 ? 'badge-warning' : 'badge-info'">{{ p.current_queue_count }}</span>
          </td>
          <td>{{ p.avg_wait_minutes }}</td>
          <td>
            <span class="badge" :class="p.is_active ? 'badge-success' : 'badge-gray'">{{ p.is_active ? '启用' : '停用' }}</span>
          </td>
        </tr>
        <tr v-if="pits.length === 0">
          <td colspan="5" class="text-center text-sm" style="padding: 24px;">暂无坑口数据</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
