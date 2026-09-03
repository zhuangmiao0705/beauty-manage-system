<script setup lang="ts">
import { salonStore } from '../data/repository'
import { currency } from '../utils'

withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
  }>(),
  { placeholder: '输入会员姓名或手机号搜索' }
)
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
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
      v-for="member in salonStore.members"
      :key="member.id"
      :label="`${member.name} · ${member.phone}（本金 ${currency(member.principalBalance)}，赠送 ${currency(member.giftBalance)}）`"
      :value="member.id"
    />
  </el-select>
</template>
