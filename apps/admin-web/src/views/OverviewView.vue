<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { dashboardApi, alertApi } from '../api/client'

interface Overview {
  today_total_waybills: number
  today_completed: number
  today_cancelled: number
  in_progress: number
  today_total_tonnage: number
  pit_summaries: Array<{ pit_id: string; pit_name: string; current_queue: number; avg_wait_minutes: number; today_trips: number; today_tonnage: number }>
  date: string
}

interface Alert {
  id: string; waybill_id: string; type: string; severity: number; description: string; status: string; created_at: string; resolved_at: string | null
}

const overview = ref<Overview | null>(null)
const alerts = ref<Alert[]>([])
const loading = ref(true)

onMounted(async () => {
  try {
    const [o, a] = await Promise.all([
      dashboardApi.overview(),
      alertApi.list({ status: 'open' }),
    ])
    overview.value = o
    alerts.value = a.slice(0, 5)
  } catch (e) {
    console.error('failed to load overview', e)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div v-if="loading" class="text-center" style="padding: 60px;">加载中...</div>

  <template v-else-if="overview">
    <div class="stats-grid">
      <div class="stat-card">
        <div class="label">今日总运单</div>
        <div class="value">{{ overview.today_total_waybills }}</div>
      </div>
      <div class="stat-card">
        <div class="label">已完成</div>
        <div class="value" style="color: var(--success);">{{ overview.today_completed }}</div>
      </div>
      <div class="stat-card">
        <div class="label">进行中</div>
        <div class="value" style="color: var(--primary);">{{ overview.in_progress }}</div>
      </div>
      <div class="stat-card">
        <div class="label">今日吨位</div>
        <div class="value">{{ overview.today_total_tonnage.toFixed(1) }} <span style="font-size: 14px; font-weight: 400; color: var(--gray-500);">吨</span></div>
      </div>
      <div class="stat-card">
        <div class="label">已取消</div>
        <div class="value" style="color: var(--danger);">{{ overview.today_cancelled }}</div>
      </div>
    </div>

    <div class="card" style="margin-bottom: 24px;">
      <div class="section-header">
        <h2>坑口实时状态</h2>
        <span class="text-sm">日期: {{ overview.date }}</span>
      </div>
      <table>
        <thead>
          <tr>
            <th>坑口名称</th>
            <th>当前排队</th>
            <th>平均等待(分钟)</th>
            <th>今日趟次</th>
            <th>今日吨位</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in overview.pit_summaries" :key="p.pit_id">
            <td><strong>{{ p.pit_name }}</strong></td>
            <td><span class="badge" :class="p.current_queue > 5 ? 'badge-warning' : 'badge-info'">{{ p.current_queue }}</span></td>
            <td>{{ p.avg_wait_minutes }}</td>
            <td>{{ p.today_trips }}</td>
            <td>{{ p.today_tonnage.toFixed(1) }}</td>
          </tr>
          <tr v-if="overview.pit_summaries.length === 0">
            <td colspan="5" class="text-center text-sm" style="padding: 24px;">暂无坑口数据</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="card">
      <div class="section-header">
        <h2>待处理告警</h2>
        <router-link to="/alerts" class="text-sm">查看全部 →</router-link>
      </div>
      <table>
        <thead>
          <tr>
            <th>类型</th>
            <th>描述</th>
            <th>严重程度</th>
            <th>时间</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="a in alerts" :key="a.id">
            <td><span class="badge badge-danger">{{ a.type }}</span></td>
            <td>{{ a.description }}</td>
            <td><span class="badge" :class="a.severity > 2 ? 'badge-danger' : 'badge-warning'">{{ a.severity }}</span></td>
            <td class="text-sm">{{ new Date(a.created_at).toLocaleString() }}</td>
          </tr>
          <tr v-if="alerts.length === 0">
            <td colspan="4" class="text-center text-sm" style="padding: 24px;">暂无待处理告警 ✅</td>
          </tr>
        </tbody>
      </table>
    </div>
  </template>
</template>
