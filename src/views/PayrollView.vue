<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import type { FormInstance } from 'element-plus'
import { ClipboardList, ReceiptText, Save, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { DEFAULT_ATTENDANCE_REST_DAYS } from '../config/options'
import { salonStore, upsertAttendance } from '../data/repository'
import {
  buildHandworkDetails,
  buildPerformanceDetails,
  buildSalaryRows,
  employeeActiveDays
} from '../services/payroll'
import type { AppSnapshot, Employee, EmployeeSalaryRow } from '../types'
import { currency, fullDateTime, localMonthKey } from '../utils'
import { errorMessage, notify } from '../utils/feedback'

type SalaryFilters = { employeeName: string; month: string }

const activeTab = ref<'salary' | 'attendance'>('salary')
const salaryFilterForm = reactive<SalaryFilters>({ employeeName: '', month: localMonthKey() })
const appliedSalaryFilters = reactive<SalaryFilters>({ ...salaryFilterForm })
const attendanceFilterForm = reactive<SalaryFilters>({ employeeName: '', month: localMonthKey() })
const appliedAttendanceFilters = reactive<SalaryFilters>({ ...attendanceFilterForm })
const attendanceDrafts = reactive<Record<string, number>>({})
const savingAttendance = ref('')
const attendanceFormRef = ref<FormInstance>()
const detailModal = ref<'handwork' | 'performance' | null>(null)
const selectedSalaryRow = ref<EmployeeSalaryRow | null>(null)
const snapshot = salonStore as unknown as AppSnapshot

const salaryRows = computed(() =>
  buildSalaryRows(snapshot, appliedSalaryFilters.month).filter(item => {
    const name = appliedSalaryFilters.employeeName.trim()
    return !name || item.name.includes(name)
  })
)
const attendanceRows = computed(() =>
  salonStore.employees
    .filter(employee => {
      const name = appliedAttendanceFilters.employeeName.trim()
      return !name || employee.name.includes(name)
    })
    .map(employee => ({
      ...employee,
      activeDays: employeeActiveDays(
        snapshot,
        employee as Employee,
        appliedAttendanceFilters.month
      ),
      updatedAt: salonStore.attendanceRecords.find(
        item => item.employeeId === employee.id && item.month === appliedAttendanceFilters.month
      )?.updatedAt
    }))
    .filter(item => item.activeDays > 0)
)
const handworkDetails = computed(() =>
  selectedSalaryRow.value
    ? buildHandworkDetails(snapshot, selectedSalaryRow.value.employeeId, appliedSalaryFilters.month)
    : []
)
const performanceDetails = computed(() =>
  selectedSalaryRow.value
    ? buildPerformanceDetails(
        snapshot,
        selectedSalaryRow.value.employeeId,
        appliedSalaryFilters.month
      )
    : []
)
const handworkCommission = computed(() =>
  handworkDetails.value.reduce((sum, item) => sum + item.commission, 0)
)
const {
  currentPage: salaryPage,
  pageSize: salaryPageSize,
  paginatedRecords: paginatedSalaryRows
} = useTablePagination(salaryRows)
const {
  currentPage: attendancePage,
  pageSize: attendancePageSize,
  paginatedRecords: paginatedAttendanceRows
} = useTablePagination(attendanceRows)
const {
  currentPage: handworkPage,
  pageSize: handworkPageSize,
  paginatedRecords: paginatedHandworkDetails
} = useTablePagination(handworkDetails)
const {
  currentPage: performancePage,
  pageSize: performancePageSize,
  paginatedRecords: paginatedPerformanceDetails
} = useTablePagination(performanceDetails)

function employeeColor(employeeId: string) {
  return salonStore.employees.find(employee => employee.id === employeeId)?.color ?? '#d77c8d'
}

function loadAttendanceDrafts() {
  attendanceRows.value.forEach(employee => {
    attendanceDrafts[employee.id] =
      salonStore.attendanceRecords.find(
        item => item.employeeId === employee.id && item.month === appliedAttendanceFilters.month
      )?.restDays ?? DEFAULT_ATTENDANCE_REST_DAYS
  })
}

function querySalary() {
  Object.assign(appliedSalaryFilters, salaryFilterForm)
}

function resetSalary() {
  Object.assign(salaryFilterForm, { employeeName: '', month: localMonthKey() })
  querySalary()
}

function queryAttendance() {
  Object.assign(appliedAttendanceFilters, attendanceFilterForm)
  loadAttendanceDrafts()
}

function resetAttendance() {
  Object.assign(attendanceFilterForm, { employeeName: '', month: localMonthKey() })
  queryAttendance()
}

function openSalaryDetail(row: EmployeeSalaryRow, type: 'handwork' | 'performance') {
  selectedSalaryRow.value = row
  detailModal.value = type
}

async function saveAttendance(employeeId: string) {
  try {
    await attendanceFormRef.value?.validateField(employeeId)
  } catch {
    return
  }
  savingAttendance.value = employeeId
  try {
    await upsertAttendance({
      employeeId,
      month: appliedAttendanceFilters.month,
      restDays: attendanceDrafts[employeeId] ?? DEFAULT_ATTENDANCE_REST_DAYS
    })
    notify('考勤记录已保存')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    savingAttendance.value = ''
  }
}

watch(activeTab, tab => {
  if (tab === 'attendance') queryAttendance()
})
loadAttendanceDrafts()
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <el-radio-group v-model="activeTab" class="salon-radio-group">
        <el-radio-button value="salary">员工薪水</el-radio-button>
        <el-radio-button value="attendance">考勤管理</el-radio-button>
      </el-radio-group>
    </section>

    <section v-if="activeTab === 'salary'" class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <label class="table-filter-field">
          <span>员工姓名</span>
          <el-input
            v-model="salaryFilterForm.employeeName"
            clearable
            class="table-filter-input"
            placeholder="请输入员工姓名"
          />
        </label>
        <label class="table-filter-field">
          <span>月份</span>
          <el-date-picker
            v-model="salaryFilterForm.month"
            type="month"
            class="service-date-picker"
            format="YYYY年MM月"
            value-format="YYYY-MM"
            placeholder="请选择月份"
            :clearable="false"
          />
        </label>
        <div class="service-filter-actions">
          <el-button type="primary" @click="querySalary">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetSalary">重置</el-button>
        </div>
      </div>
      <el-table :data="paginatedSalaryRows" row-key="employeeId" stripe class="salon-table">
        <el-table-column label="姓名" min-width="160" fixed="left">
          <template #default="{ row }">
            <div class="employee-inline">
              <span :style="{ background: employeeColor(row.employeeId) }">
                {{ row.name.slice(-1) }}
              </span>
              <b>{{ row.name }}</b>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="会员充值" min-width="115">
          <template #default="{ row }">{{ currency(row.rechargePerformance) }}</template>
        </el-table-column>
        <el-table-column label="套餐额外支付" min-width="125">
          <template #default="{ row }">{{ currency(row.packagePurchasePerformance) }}</template>
        </el-table-column>
        <el-table-column label="绩效总金额" min-width="125">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.totalPerformance) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="套盒手工" min-width="95">
          <template #default="{ row }">{{ row.packageServiceCount }} 次</template>
        </el-table-column>
        <el-table-column label="普通手工" min-width="95">
          <template #default="{ row }">{{ row.normalServiceCount }} 次</template>
        </el-table-column>
        <el-table-column label="底薪" min-width="105">
          <template #default="{ row }">{{ currency(row.baseSalary, 2) }}</template>
        </el-table-column>
        <el-table-column label="提成" min-width="105">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.commission, 2) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="餐补" min-width="145">
          <template #default="{ row }">
            {{ currency(row.mealAllowance, 2) }}（{{ row.mealDays }}天）
          </template>
        </el-table-column>
        <el-table-column label="全勤" min-width="105">
          <template #default="{ row }">{{ currency(row.attendanceBonus, 2) }}</template>
        </el-table-column>
        <el-table-column label="总收入" min-width="115" fixed="right">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.totalIncome, 2) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="205" fixed="right">
          <template #default="{ row }">
            <div class="element-row-actions">
              <el-button
                size="small"
                @click="openSalaryDetail(row as EmployeeSalaryRow, 'handwork')"
              >
                <ClipboardList :size="14" />
                手工明细
              </el-button>
              <el-button
                size="small"
                @click="openSalaryDetail(row as EmployeeSalaryRow, 'performance')"
              >
                <ReceiptText :size="14" />
                绩效明细
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="salaryPage"
        v-model:page-size="salaryPageSize"
        :total="salaryRows.length"
      />
    </section>

    <section v-else class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <label class="table-filter-field">
          <span>员工姓名</span>
          <el-input
            v-model="attendanceFilterForm.employeeName"
            clearable
            class="table-filter-input"
            placeholder="请输入员工姓名"
          />
        </label>
        <label class="table-filter-field">
          <span>月份</span>
          <el-date-picker
            v-model="attendanceFilterForm.month"
            type="month"
            class="service-date-picker"
            format="YYYY年MM月"
            value-format="YYYY-MM"
            placeholder="请选择月份"
            :clearable="false"
          />
        </label>
        <div class="service-filter-actions">
          <el-button type="primary" @click="queryAttendance">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetAttendance">重置</el-button>
        </div>
        <span class="result-count">默认每月休息 {{ DEFAULT_ATTENDANCE_REST_DAYS }} 天</span>
      </div>
      <el-form ref="attendanceFormRef" :model="attendanceDrafts">
        <el-table :data="paginatedAttendanceRows" row-key="id" stripe class="salon-table">
          <el-table-column label="姓名" min-width="160">
            <template #default="{ row }">
              <div class="employee-inline">
                <span :style="{ background: row.color }">{{ row.name.slice(-1) }}</span>
                <b>{{ row.name }}</b>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="月份" min-width="110">
            <template #default>{{ appliedAttendanceFilters.month }}</template>
          </el-table-column>
          <el-table-column prop="activeDays" label="在职自然日" min-width="110" />
          <el-table-column label="休息天数" min-width="180">
            <template #default="{ row }">
              <el-form-item
                :prop="row.id"
                :rules="[
                  {
                    required: true,
                    type: 'number',
                    min: 0,
                    max: row.activeDays,
                    message: `请输入0至${row.activeDays}天`,
                    trigger: 'change'
                  }
                ]"
                class="attendance-form-item"
                inline-message
              >
                <el-input-number
                  v-model="attendanceDrafts[row.id]"
                  :min="0"
                  :max="row.activeDays"
                  :precision="0"
                  size="small"
                />
              </el-form-item>
            </template>
          </el-table-column>
          <el-table-column label="更新时间" min-width="170">
            <template #default="{ row }">
              {{ row.updatedAt ? fullDateTime(row.updatedAt) : '尚未保存，使用默认值' }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="100" fixed="right">
            <template #default="{ row }">
              <el-button
                type="primary"
                size="small"
                :loading="savingAttendance === row.id"
                @click="saveAttendance(row.id)"
              >
                <Save :size="14" />
                保存
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-form>
      <TablePagination
        v-model="attendancePage"
        v-model:page-size="attendancePageSize"
        :total="attendanceRows.length"
      />
    </section>

    <BaseModal
      v-if="detailModal === 'handwork' && selectedSalaryRow"
      title="手工明细"
      :subtitle="`${selectedSalaryRow.name} · ${appliedSalaryFilters.month}`"
      wide
      @close="detailModal = null"
    >
      <el-descriptions :column="3" border class="payroll-detail-summary">
        <el-descriptions-item label="普通手工">
          {{ selectedSalaryRow.normalServiceCount }} 次
        </el-descriptions-item>
        <el-descriptions-item label="套盒手工">
          {{ selectedSalaryRow.packageServiceCount }} 次
        </el-descriptions-item>
        <el-descriptions-item label="手工提成">
          <strong class="money">{{ currency(handworkCommission, 2) }}</strong>
        </el-descriptions-item>
      </el-descriptions>
      <el-table :data="paginatedHandworkDetails" row-key="id" stripe max-height="430">
        <el-table-column label="服务时间" min-width="165">
          <template #default="{ row }">{{ fullDateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="顾客" min-width="130">
          <template #default="{ row }">
            <div>{{ row.memberName }}</div>
            <div v-if="row.memberPhone" class="service-customer-phone">
              {{ row.memberPhone }}
            </div>
          </template>
        </el-table-column>
        <el-table-column label="手工类型" min-width="105">
          <template #default="{ row }">
            <el-tag round :type="row.serviceType === '套盒手工' ? 'warning' : 'info'">
              {{ row.serviceType }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="serviceName" label="服务项目" min-width="145" />
        <el-table-column label="服务时长" min-width="95">
          <template #default="{ row }">{{ row.duration }} 分钟</template>
        </el-table-column>
        <el-table-column label="本次提成" min-width="105" fixed="right">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.commission, 2) }}</strong>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="handworkPage"
        v-model:page-size="handworkPageSize"
        :total="handworkDetails.length"
      />
    </BaseModal>

    <BaseModal
      v-if="detailModal === 'performance' && selectedSalaryRow"
      title="绩效明细"
      :subtitle="`${selectedSalaryRow.name} · ${appliedSalaryFilters.month}`"
      wide
      @close="detailModal = null"
    >
      <el-descriptions :column="3" border class="payroll-detail-summary">
        <el-descriptions-item label="会员充值">
          {{ currency(selectedSalaryRow.rechargePerformance) }}
        </el-descriptions-item>
        <el-descriptions-item label="套餐额外支付">
          {{ currency(selectedSalaryRow.packagePurchasePerformance) }}
        </el-descriptions-item>
        <el-descriptions-item label="绩效总金额">
          <strong class="money">{{ currency(selectedSalaryRow.totalPerformance) }}</strong>
        </el-descriptions-item>
      </el-descriptions>
      <el-table :data="paginatedPerformanceDetails" row-key="id" stripe max-height="430">
        <el-table-column label="发生时间" min-width="165">
          <template #default="{ row }">{{ fullDateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="会员信息" min-width="135">
          <template #default="{ row }">
            <div>{{ row.memberName }}</div>
            <div v-if="row.memberPhone" class="service-customer-phone">
              {{ row.memberPhone }}
            </div>
          </template>
        </el-table-column>
        <el-table-column label="绩效来源" min-width="125">
          <template #default="{ row }">
            <el-tag round :type="row.source === '会员充值' ? 'success' : 'warning'">
              {{ row.source }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="businessName" label="业务内容" min-width="145" />
        <el-table-column label="业务金额" min-width="105">
          <template #default="{ row }">{{ currency(row.businessAmount) }}</template>
        </el-table-column>
        <el-table-column label="赠送金额" min-width="105">
          <template #default="{ row }">{{ currency(row.giftAmount) }}</template>
        </el-table-column>
        <el-table-column label="余额支付" min-width="105">
          <template #default="{ row }">{{ currency(row.balancePaymentAmount) }}</template>
        </el-table-column>
        <el-table-column prop="paymentMethod" label="支付方式" min-width="145" />
        <el-table-column label="计入绩效" min-width="110" fixed="right">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.performanceAmount) }}</strong>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="performancePage"
        v-model:page-size="performancePageSize"
        :total="performanceDetails.length"
      />
    </BaseModal>
  </div>
</template>

<style scoped>
.payroll-detail-summary {
  margin-bottom: 16px;
}
</style>
