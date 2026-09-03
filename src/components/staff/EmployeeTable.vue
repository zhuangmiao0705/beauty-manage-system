<script setup lang="ts">
import { computed } from 'vue'
import TablePagination from '../TablePagination.vue'
import { useTablePagination } from '../../composables/useTablePagination'
import type { Employee } from '../../types'
import { dateOnly } from '../../utils'

const props = defineProps<{ employees: Employee[] }>()
const emit = defineEmits<{ toggle: [employee: Employee] }>()
const employeeRecords = computed(() => props.employees)
const {
  currentPage,
  pageSize,
  paginatedRecords: paginatedEmployees
} = useTablePagination(employeeRecords)
</script>

<template>
  <el-table :data="paginatedEmployees" row-key="id" stripe class="salon-table">
    <el-table-column label="员工" min-width="160">
      <template #default="{ row }">
        <div class="employee-inline">
          <span :style="{ background: row.color }">{{ row.name.slice(-1) }}</span>
          <b>{{ row.name }}</b>
        </div>
      </template>
    </el-table-column>
    <el-table-column prop="role" label="岗位" min-width="140" />
    <el-table-column label="入职时间" min-width="130">
      <template #default="{ row }">{{ dateOnly(row.createdAt) }}</template>
    </el-table-column>
    <el-table-column label="状态" width="100">
      <template #default="{ row }">
        <el-tag round :type="row.status === 'active' ? 'success' : 'info'">
          {{ row.status === 'active' ? '在职' : '停用' }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column label="操作" width="110" fixed="right">
      <template #default="{ row }">
        <el-button
          size="small"
          :type="row.status === 'active' ? 'danger' : 'success'"
          plain
          @click="emit('toggle', row as Employee)"
        >
          {{ row.status === 'active' ? '停用' : '启用' }}
        </el-button>
      </template>
    </el-table-column>
  </el-table>
  <TablePagination v-model="currentPage" v-model:page-size="pageSize" :total="employees.length" />
</template>
