<script setup lang="ts">
defineProps<{
  visible: boolean
  title?: string
  width?: string
}>()

const emit = defineEmits<{
  close: []
}>()
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="emit('close')">
      <div class="modal-content" :style="width ? { maxWidth: width } : {}">
        <div class="modal-header" v-if="title || $slots.header">
          <slot name="header">
            <h3>{{ title }}</h3>
          </slot>
          <button class="modal-close" @click="emit('close')">&times;</button>
        </div>
        <div class="modal-body">
          <slot />
        </div>
        <div class="modal-footer" v-if="$slots.footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.4);
  display: flex; align-items: center; justify-content: center; z-index: 1000;
}
.modal-content {
  background: #fff; border-radius: 12px; width: 90%; max-width: 600px;
  max-height: 85vh; overflow-y: auto; box-shadow: 0 20px 60px rgba(0,0,0,0.2);
}
.modal-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px 20px; border-bottom: 1px solid #e5e7eb;
}
.modal-header h3 { margin: 0; font-size: 16px; }
.modal-close {
  background: none; border: none; font-size: 24px; cursor: pointer; color: #888;
  padding: 0 4px; line-height: 1;
}
.modal-close:hover { color: #333; }
.modal-body { padding: 20px; }
.modal-footer {
  padding: 12px 20px; border-top: 1px solid #e5e7eb;
  display: flex; justify-content: flex-end; gap: 8px;
}
</style>
