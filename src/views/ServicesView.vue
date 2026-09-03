<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import 'element-plus/es/components/message-box/style/css'
import type { FormInstance, FormItemRule, FormRules } from 'element-plus'
import { useRoute } from 'vue-router'
import { Clock3, Filter, Plus, Scissors, Search, Sparkles } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import EmployeeSelect from '../components/EmployeeSelect.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { authStore } from '../auth'
import { RECHARGE_PAYMENT_METHODS, SERVICE_TYPE_LABELS, SERVICE_TYPES } from '../config/options'
import { addService, cancelService, salonStore } from '../data/repository'
import type { ExternalPaymentMethod, ServiceInput, ServiceType } from '../types'
import { currency, dateTime, isToday, localDateKey } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { validateForm } from '../utils/validation'

type ServiceForm = ServiceInput & { customerType: 'member' | 'guest' }
type ServiceFilters = {
  memberPhone: string
  memberName: string
  employee: string
  serviceType: ServiceType | ''
  date: string
}
type RuleCallback = (error?: Error) => void

const route = useRoute()
const dateFilter = ref('today')
const modal = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()
const emptyFilters = (): ServiceFilters => ({
  memberPhone: '',
  memberName: '',
  employee: '',
  serviceType: '',
  date: ''
})
const filterForm = reactive<ServiceFilters>(emptyFilters())
const appliedFilters = reactive<ServiceFilters>(emptyFilters())
const form = reactive<ServiceForm>({
  customerType: 'member',
  requestId: crypto.randomUUID(),
  memberId: '',
  guestName: '游客',
  employee: '',
  projectId: '',
  externalPaymentMethod: ''
})
const memberRule: FormItemRule = {
  required: true,
  trigger: 'change',
  validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
    if (form.customerType !== 'member' || (typeof value === 'string' && value)) callback()
    else callback(new Error('请选择会员'))
  }
}
const guestRule: FormItemRule = {
  required: true,
  trigger: ['blur', 'change'],
  validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
    if (form.customerType !== 'guest' || (typeof value === 'string' && value.trim())) callback()
    else callback(new Error('请输入游客称呼'))
  }
}
const rules: FormRules<ServiceForm> = {
  customerType: [{ required: true, message: '请选择消费对象', trigger: 'change' }],
  memberId: [memberRule],
  guestName: [guestRule],
  employee: [{ required: true, message: '请选择服务员工', trigger: 'change' }],
  projectId: [{ required: true, message: '请选择消费项目', trigger: 'change' }],
  externalPaymentMethod: [
    {
      required: true,
      trigger: 'change',
      validator: (_rule, value, callback) => {
        if (!requiresExternalPayment.value || value) callback()
        else callback(new Error('请选择实际支付方式'))
      }
    }
  ]
}

const memberPhoneById = computed(
  () => new Map(salonStore.members.map(member => [member.id, member.phone]))
)
const selectedProject = computed(() => salonStore.projects.find(item => item.id === form.projectId))
const selectedMember = computed(() => salonStore.members.find(item => item.id === form.memberId))
const balancePaymentAmount = computed(() => {
  if (form.customerType !== 'member' || !selectedProject.value || !selectedMember.value) return 0
  return Math.min(selectedMember.value.balance, selectedProject.value.price)
})
const externalPaymentAmount = computed(() =>
  selectedProject.value ? Math.max(0, selectedProject.value.price - balancePaymentAmount.value) : 0
)
const requiresExternalPayment = computed(
  () =>
    form.customerType === 'guest' ||
    (form.customerType === 'member' && !!selectedMember.value && externalPaymentAmount.value > 0)
)
function servicePhone(memberId: string | null) {
  return memberId ? (memberPhoneById.value.get(memberId) ?? '') : ''
}

function serviceTypeLabel(serviceType?: ServiceType) {
  return SERVICE_TYPE_LABELS[serviceType ?? '普通手工']
}

const filtered = computed(() =>
  salonStore.services.filter(item => {
    if (
      appliedFilters.memberPhone.trim() &&
      !servicePhone(item.memberId).includes(appliedFilters.memberPhone.trim())
    )
      return false
    if (
      appliedFilters.memberName.trim() &&
      !item.memberName.includes(appliedFilters.memberName.trim())
    )
      return false
    if (appliedFilters.employee && item.employee !== appliedFilters.employee) return false
    if (
      appliedFilters.serviceType &&
      (item.serviceType ?? '普通手工') !== appliedFilters.serviceType
    )
      return false
    if (appliedFilters.date) return localDateKey(item.createdAt) === appliedFilters.date
    const date = new Date(item.createdAt)
    const now = new Date()
    if (dateFilter.value === 'today') return isToday(item.createdAt)
    if (dateFilter.value === 'week')
      return date >= new Date(now.getFullYear(), now.getMonth(), now.getDate() - 6)
    if (dateFilter.value === 'month')
      return date.getMonth() === now.getMonth() && date.getFullYear() === now.getFullYear()
    return true
  })
)
const completedServices = computed(() => filtered.value.filter(item => item.status === 'completed'))
const { currentPage, pageSize, paginatedRecords: paginatedServices } = useTablePagination(filtered)
const totalAmount = computed(() =>
  completedServices.value.reduce((sum, item) => sum + item.amount, 0)
)
const totalMinutes = computed(() =>
  completedServices.value.reduce((sum, item) => sum + item.duration, 0)
)

function queryServices() {
  Object.assign(appliedFilters, filterForm)
  if (filterForm.date) dateFilter.value = 'all'
}

function resetFilters() {
  Object.assign(filterForm, emptyFilters())
  Object.assign(appliedFilters, emptyFilters())
  dateFilter.value = 'today'
}

function openModal() {
  Object.assign(form, {
    customerType: salonStore.members.length ? 'member' : 'guest',
    requestId: crypto.randomUUID(),
    memberId: '',
    guestName: '游客',
    employee: '',
    projectId: '',
    externalPaymentMethod: ''
  })
  modal.value = true
}
watch(
  () => [form.customerType, form.memberId, form.projectId],
  () => {
    form.externalPaymentMethod = ''
    formRef.value?.clearValidate(['memberId', 'guestName', 'externalPaymentMethod'])
  }
)
watch(dateFilter, value => {
  if (value !== 'all') {
    filterForm.date = ''
    appliedFilters.date = ''
  }
})
watch(
  () => route.query.action,
  action => {
    if (action === 'new') openModal()
  },
  { immediate: true }
)

async function submit() {
  if (saving.value || !(await validateForm(formRef.value))) return
  saving.value = true
  try {
    await addService({
      memberId: form.customerType === 'guest' ? '' : form.memberId,
      guestName: form.customerType === 'guest' ? form.guestName.trim() : '',
      employee: form.employee,
      projectId: form.projectId,
      externalPaymentMethod: form.externalPaymentMethod,
      requestId: form.requestId
    })
    modal.value = false
    notify('服务记录已保存')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function cancelRecord(serviceId: string) {
  try {
    await ElMessageBox.confirm(
      '撤销后将冲减消费营业额，并按原扣款拆分退回会员赠送余额和本金余额。确认继续吗？',
      '确认撤销服务',
      { type: 'warning', confirmButtonText: '确认撤销', cancelButtonText: '取消' }
    )
    await cancelService(serviceId)
    notify('服务已撤销，相关余额和营业额已恢复')
  } catch (reason) {
    if (reason === 'cancel' || reason === 'close') return
    notify(errorMessage(reason, '撤销失败'), 'error')
  }
}
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <el-radio-group v-model="dateFilter" class="salon-radio-group">
        <el-radio-button value="today">今日</el-radio-button>
        <el-radio-button value="week">近7天</el-radio-button>
        <el-radio-button value="month">本月</el-radio-button>
        <el-radio-button value="all">全部</el-radio-button>
      </el-radio-group>
      <el-button type="primary" @click="openModal">
        <Plus :size="17" />
        登记消费
      </el-button>
    </section>
    <section class="summary-strip">
      <div>
        <span class="summary-icon rose"><Sparkles :size="20" /></span>
        <p>
          服务人次
          <strong>{{ completedServices.length }}</strong>
          <span>当前筛选范围</span>
        </p>
      </div>
      <div>
        <span class="summary-icon gold"><Scissors :size="20" /></span>
        <p>
          服务金额
          <strong>{{ currency(totalAmount) }}</strong>
          <span>已完成服务合计</span>
        </p>
      </div>
      <div>
        <span class="summary-icon green"><Clock3 :size="20" /></span>
        <p>
          服务时长
          <strong>{{ Math.round((totalMinutes / 60) * 10) / 10 }} 小时</strong>
          <span>员工投入时间</span>
        </p>
      </div>
    </section>
    <section class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar service-filter-toolbar">
        <label class="table-filter-field">
          <span>会员手机号</span>
          <el-input
            v-model="filterForm.memberPhone"
            clearable
            class="service-filter-input"
            placeholder="请输入会员手机号"
            @keyup.enter="queryServices"
          />
        </label>
        <label class="table-filter-field">
          <span>会员名</span>
          <el-input
            v-model="filterForm.memberName"
            clearable
            class="service-filter-input"
            placeholder="请输入会员名"
            @keyup.enter="queryServices"
          />
        </label>
        <label class="table-filter-field">
          <span>员工</span>
          <el-select
            v-model="filterForm.employee"
            clearable
            class="service-filter-select"
            placeholder="请选择员工"
          >
            <el-option
              v-for="employee in salonStore.employees"
              :key="employee.id"
              :label="employee.name"
              :value="employee.name"
            />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>消费类型</span>
          <el-select
            v-model="filterForm.serviceType"
            clearable
            class="service-filter-select"
            placeholder="请选择消费类型"
          >
            <el-option
              v-for="type in SERVICE_TYPES"
              :key="type"
              :label="SERVICE_TYPE_LABELS[type]"
              :value="type"
            />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>服务日期</span>
          <el-date-picker
            v-model="filterForm.date"
            class="service-date-picker"
            type="date"
            placeholder="请选择服务日期"
            format="YYYY年MM月DD日"
            value-format="YYYY-MM-DD"
            clearable
          />
        </label>
        <div class="service-filter-actions">
          <el-button type="primary" @click="queryServices">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetFilters">重置</el-button>
        </div>
        <el-tag round type="info" class="service-record-count">
          <Filter :size="14" />
          {{ filtered.length }} 条服务记录
        </el-tag>
      </div>
      <el-table :data="paginatedServices" row-key="id" stripe class="salon-table">
        <el-table-column label="服务时间" min-width="130">
          <template #default="{ row }">{{ dateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="顾客" min-width="140">
          <template #default="{ row }">
            <div class="service-customer">
              <div>{{ row.memberName }}</div>
              <div v-if="servicePhone(row.memberId)" class="service-customer-phone">
                {{ servicePhone(row.memberId) }}
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="消费项目" min-width="120">
          <template #default="{ row }">
            <span class="service-name">
              <Sparkles :size="15" />
              {{ row.serviceName }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="消费类型" min-width="110">
          <template #default="{ row }">
            <el-tag :type="row.serviceType === '套盒手工' ? 'warning' : 'info'" round>
              {{ serviceTypeLabel(row.serviceType) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="服务员工" min-width="120">
          <template #default="{ row }">
            <div class="employee-inline">
              <span>{{ row.employee.slice(-1) }}</span>
              {{ row.employee }}
            </div>
          </template>
        </el-table-column>
        <el-table-column label="服务时长" min-width="100">
          <template #default="{ row }">{{ row.duration }} 分钟</template>
        </el-table-column>
        <el-table-column label="服务金额" min-width="110">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.amount) }}</strong>
          </template>
        </el-table-column>
        <el-table-column prop="paymentMethod" label="支付方式" min-width="155" />
        <el-table-column label="状态" min-width="90">
          <template #default="{ row }">
            <el-tag round :type="row.status === 'completed' ? 'success' : 'info'">
              {{ row.status === 'completed' ? '已完成' : '已撤销' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          v-if="authStore.user?.role === 'manager'"
          label="操作"
          width="95"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              size="small"
              type="danger"
              plain
              :disabled="row.status !== 'completed' || !row.transactionId"
              @click="cancelRecord(row.id)"
            >
              撤销
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="currentPage"
        v-model:page-size="pageSize"
        :total="filtered.length"
      />
      <div v-if="!filtered.length" class="empty-state">
        <Sparkles :size="28" />
        <b>暂无服务记录</b>
        <span>点击“登记消费”添加第一条记录</span>
      </div>
    </section>

    <BaseModal
      v-if="modal"
      title="登记普通消费"
      subtitle="记录本次普通消费，自动计入消费营业额、员工服务与提成统计"
      @close="modal = false"
    >
      <el-form ref="formRef" :model="form" :rules="rules" scroll-to-error label-position="top">
        <div class="form-grid">
          <el-form-item label="服务员工" prop="employee">
            <EmployeeSelect v-model="form.employee" />
          </el-form-item>
          <el-form-item label="消费对象" prop="customerType">
            <el-radio-group v-model="form.customerType">
              <el-radio-button value="member">会员</el-radio-button>
              <el-radio-button value="guest">游客</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item
            v-if="form.customerType === 'member'"
            class="span-2"
            label="选择会员"
            prop="memberId"
          >
            <el-select v-model="form.memberId" filterable>
              <el-option
                v-for="member in salonStore.members"
                :key="member.id"
                :label="`${member.name} · ${member.phone} · 账户余额 ${currency(member.balance)}`"
                :value="member.id"
              />
            </el-select>
          </el-form-item>
          <el-form-item v-else class="span-2" label="游客称呼" prop="guestName">
            <el-input v-model="form.guestName" clearable placeholder="例如：游客、王女士" />
          </el-form-item>
          <el-form-item class="span-2" label="消费项目" prop="projectId">
            <el-select v-model="form.projectId" filterable clearable placeholder="请选择消费项目">
              <el-option
                v-for="item in salonStore.projects.filter(project => project.status === 'active')"
                :key="item.id"
                :label="`${item.name} · ${currency(item.price)} · ${item.duration}分钟`"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
          <el-form-item
            v-if="requiresExternalPayment && selectedProject"
            class="span-2"
            label="实际支付方式"
            prop="externalPaymentMethod"
          >
            <el-select v-model="form.externalPaymentMethod" placeholder="请选择支付方式">
              <el-option
                v-for="item in RECHARGE_PAYMENT_METHODS"
                :key="item"
                :label="item"
                :value="item"
              />
            </el-select>
          </el-form-item>
        </div>
      </el-form>
      <div v-if="selectedProject" class="form-tip">
        项目价格 {{ currency(selectedProject.price) }}，服务时长
        {{ selectedProject.duration }}分钟；本次余额支付
        {{ currency(balancePaymentAmount) }}，实际支付
        {{ currency(externalPaymentAmount) }}。保存后将自动确认消费营业额。
      </div>
      <template #footer>
        <el-button @click="modal = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submit">保存记录</el-button>
      </template>
    </BaseModal>
  </div>
</template>
