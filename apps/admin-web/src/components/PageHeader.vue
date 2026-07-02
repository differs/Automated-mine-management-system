<script setup lang="ts">
defineProps<{
  title: string
}>()
</script>

<template>
  <div class="section-header">
    <h2>{{ title }}</h2>
    <div class="flex gap-2 items-center">
      <slot name="actions" />
    </div>
  </div>
</template>
