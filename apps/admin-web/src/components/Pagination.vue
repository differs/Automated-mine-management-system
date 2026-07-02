<script setup lang="ts">
defineProps<{
  page: number
  pageSize: number
  total: number
  totalPages: number
}>()

const emit = defineEmits<{
  change: [page: number]
}>()
</script>

<template>
  <div class="pagination" v-if="totalPages > 1">
    <button class="page-btn" :disabled="page <= 1" @click="emit('change', page - 1)">&laquo; 上一页</button>
    <span class="page-info">第 {{ page }} / {{ totalPages }} 页，共 {{ total }} 条</span>
    <button class="page-btn" :disabled="page >= totalPages" @click="emit('change', page + 1)">下一页 &raquo;</button>
  </div>
</template>

<style scoped>
.pagination {
  display: flex; align-items: center; justify-content: center;
  gap: 12px; padding: 16px 0; font-size: 14px; color: #666;
}
.page-btn {
  padding: 6px 12px; border: 1px solid #d1d5db; border-radius: 6px;
  background: #fff; cursor: pointer; font-size: 13px;
}
.page-btn:hover:not(:disabled) { background: #f3f4f6; }
.page-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.page-info { color: #555; }
</style>
