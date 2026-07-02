<script setup lang="ts" generic="T">
defineProps<{
  columns: Array<{ key: string; label: string; width?: string }>
  data: T[]
  loading?: boolean
  emptyText?: string
}>()

defineEmits<{
  rowClick: [row: T]
}>()
</script>

<template>
  <div class="table-wrap">
    <div v-if="loading" class="loading-state">加载中...</div>
    <table v-else-if="data.length" class="table">
      <thead>
        <tr>
          <th v-for="col in columns" :key="col.key" :style="col.width ? { width: col.width } : {}">
            {{ col.label }}
          </th>
          <slot name="header-extra" />
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, idx) in data" :key="idx" @click="$emit('rowClick', row)" class="clickable">
          <td v-for="col in columns" :key="col.key">
            <slot :name="`cell-${col.key}`" :row="row" :value="(row as any)[col.key]">
              {{ (row as any)[col.key] }}
            </slot>
          </td>
          <slot name="row-extra" :row="row" />
        </tr>
      </tbody>
    </table>
    <div v-else class="empty-state">{{ emptyText || '暂无数据' }}</div>
  </div>
</template>

<style scoped>
.table-wrap { overflow-x: auto; }
.loading-state, .empty-state {
  padding: 32px; text-align: center; color: #888;
}
.clickable { cursor: pointer; }
.clickable:hover { background: #f5f7fa; }
</style>
