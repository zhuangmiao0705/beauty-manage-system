<script setup lang="ts">
import { computed } from 'vue'
import { salonStore } from '../data/repository'

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    activeOnly?: boolean
  }>(),
  { placeholder: '请选择员工', activeOnly: true }
)
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const employees = computed(() =>
  props.activeOnly
    ? salonStore.employees.filter(item => item.status === 'active')
    : salonStore.employees
)
</script>

<template>
  <el-select
    :model-value="modelValue"
    clearable
    filterable
    :placeholder="placeholder"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <el-option
      v-for="employee in employees"
      :key="employee.id"
      :label="`${employee.name} · ${employee.role}`"
      :value="employee.name"
    />
  </el-select>
</template>
