<script setup lang="ts">
import { computed } from 'vue'
import { KeyRound } from 'lucide-vue-next'
import TablePagination from '../TablePagination.vue'
import { useTablePagination } from '../../composables/useTablePagination'
import type { AccountRecord } from '../../types'
import { dateOnly } from '../../utils'

const props = defineProps<{ accounts: AccountRecord[] }>()
const emit = defineEmits<{
  reset: [account: AccountRecord]
  toggle: [account: AccountRecord]
}>()
const accountRecords = computed(() => props.accounts)
const {
  currentPage,
  pageSize,
  paginatedRecords: paginatedAccounts
} = useTablePagination(accountRecords)
</script>

<template>
  <el-table :data="paginatedAccounts" row-key="id" stripe class="salon-table">
    <el-table-column prop="username" label="登录账号" min-width="120" />
    <el-table-column prop="displayName" label="使用人" min-width="120" />
    <el-table-column label="角色" min-width="120">
      <template #default="{ row }">
        <el-tag round :type="row.role === 'manager' ? 'danger' : 'primary'">
          {{ row.role === 'manager' ? '店长' : '员工' }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column label="创建日期" min-width="120">
      <template #default="{ row }">{{ dateOnly(row.createdAt) }}</template>
    </el-table-column>
    <el-table-column label="状态" min-width="120">
      <template #default="{ row }">
        <el-tag round :type="row.status === 'active' ? 'success' : 'info'">
          {{ row.status === 'active' ? '正常' : '停用' }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column label="操作" min-width="170" fixed="right">
      <template #default="{ row }">
        <el-button size="small" @click="emit('reset', row as AccountRecord)">
          <KeyRound :size="14" />
          重置密码
        </el-button>
        <el-button
          size="small"
          :type="row.status === 'active' ? 'danger' : 'success'"
          plain
          @click="emit('toggle', row as AccountRecord)"
        >
          {{ row.status === 'active' ? '停用' : '启用' }}
        </el-button>
      </template>
    </el-table-column>
  </el-table>
  <TablePagination v-model="currentPage" v-model:page-size="pageSize" :total="accounts.length" />
</template>
