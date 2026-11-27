<script setup lang="ts">
import { computed } from 'vue'
import type { CommissionMode } from '../types'

const mode = defineModel<CommissionMode>('mode', { required: true })
const rate = defineModel<number>('rate', { required: true })
const percent = computed({
  get: () => Math.round(rate.value * 10000) / 100,
  set: (value: number) => {
    rate.value = Number(value || 0) / 100
  }
})
</script>

<template>
  <el-form-item label="提成方式">
    <el-select v-model="mode">
      <el-option label="按员工通用规则" value="general" />
      <el-option label="独立比例（成交时计提）" value="independent" />
    </el-select>
  </el-form-item>
  <el-form-item v-if="mode === 'independent'" label="独立提成比例（%）">
    <el-input-number v-model="percent" :min="0" :max="100" :precision="2" :controls="false" />
  </el-form-item>
  <div v-if="mode === 'independent'" class="form-tip span-2">
    账户本金支付补足提成差额，额外付款按本比例计提。赠送金额不计提，不叠加通用提成、超目标奖励及手工提成；修改仅影响新成交记录。
  </div>
</template>
