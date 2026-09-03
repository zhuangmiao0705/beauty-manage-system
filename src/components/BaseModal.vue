<script setup lang="ts">
defineProps<{ title: string; subtitle?: string; wide?: boolean }>()
defineEmits<{ close: [] }>()
</script>

<template>
  <el-dialog
    :model-value="true"
    :width="wide ? '760px' : '560px'"
    class="salon-dialog"
    append-to-body
    align-center
    destroy-on-close
    @close="$emit('close')"
  >
    <template #header>
      <div class="salon-dialog-title">
        <h2>{{ title }}</h2>
        <p v-if="subtitle">{{ subtitle }}</p>
      </div>
    </template>
    <slot />
    <template v-if="$slots.footer" #footer><slot name="footer" /></template>
  </el-dialog>
</template>
