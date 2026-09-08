<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import type { FormInstance, FormItemRule, FormRules } from 'element-plus'
import { useRoute } from 'vue-router'
import { CalendarDays, Clock3, List, Plus, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import EmployeeSelect from '../components/EmployeeSelect.vue'
import MemberSelect from '../components/MemberSelect.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import {
  APPOINTMENT_BUSINESS_HOURS,
  APPOINTMENT_STATUS_OPTIONS,
  RECHARGE_PAYMENT_METHODS,
  SERVICE_TYPE_LABELS,
  SERVICE_TYPES
} from '../config/options'
import {
  completeAppointment,
  createAppointment,
  salonStore,
  setAppointmentStatus,
  updateAppointment
} from '../data/repository'
import type {
  Appointment,
  AppointmentCompletionInput,
  AppointmentInput,
  AppointmentStatus,
  ServiceType
} from '../types'
import { buildEmployeeAvailability } from '../services/appointmentAvailability'
import {
  currency,
  fullDateTime,
  isPackagePurchaseAvailable,
  localDateKey,
  packagePurchaseLimitText
} from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { positiveNumberRule, requiredTextRule, validateForm } from '../utils/validation'

type AppointmentForm = Omit<AppointmentInput, 'startsAt'> & { startsAt: string }
type AppointmentFilters = {
  customerName: string
  customerPhone: string
  employee: string
  serviceType: ServiceType | ''
  status: AppointmentStatus | ''
  date: string
}
type RuleCallback = (error?: Error) => void

const route = useRoute()
const viewMode = ref<'list' | 'calendar' | 'availability'>('list')
const modal = ref<'appointment' | 'complete' | null>(null)
const editing = ref<Appointment | null>(null)
const completing = ref<Appointment | null>(null)
const saving = ref(false)
const formRef = ref<FormInstance>()
const completionFormRef = ref<FormInstance>()
const calendarDate = ref(new Date())
const availabilityDate = ref(localDateKey(new Date()))
const availabilityDuration = ref(60)
const emptyFilters = (): AppointmentFilters => ({
  customerName: '',
  customerPhone: '',
  employee: '',
  serviceType: '',
  status: '',
  date: ''
})
const filterForm = reactive<AppointmentFilters>(emptyFilters())
const appliedFilters = reactive<AppointmentFilters>(emptyFilters())

function nextAppointmentTime() {
  const date = new Date()
  date.setSeconds(0, 0)
  date.setMinutes(Math.ceil((date.getMinutes() + 1) / 30) * 30)
  return localDateTimeValue(date)
}

const form = reactive<AppointmentForm>({
  customerType: 'member',
  memberId: '',
  guestName: '',
  guestPhone: '',
  employee: '',
  serviceType: '普通手工',
  projectId: '',
  packagePurchaseId: '',
  startsAt: nextAppointmentTime(),
  duration: 60,
  note: ''
})
const completionForm = reactive<AppointmentCompletionInput>({
  appointmentId: '',
  duration: 60,
  externalPaymentMethod: ''
})

const conditionalRequired = (shouldValidate: () => boolean, message: string): FormItemRule => ({
  required: true,
  trigger: ['blur', 'change'],
  validator: (_rule: unknown, value: unknown, callback: RuleCallback) => {
    if (!shouldValidate() || (typeof value === 'string' && value.trim())) callback()
    else callback(new Error(message))
  }
})
const rules: FormRules<AppointmentForm> = {
  customerType: [{ required: true, message: '请选择顾客类型', trigger: 'change' }],
  memberId: [conditionalRequired(() => form.customerType === 'member', '请选择预约会员')],
  guestName: [conditionalRequired(() => form.customerType === 'guest', '请输入游客姓名')],
  guestPhone: [
    conditionalRequired(() => form.customerType === 'guest', '请输入游客手机号'),
    {
      trigger: ['blur', 'change'],
      validator: (_rule, value, callback) => {
        if (form.customerType !== 'guest' || /^1\d{10}$/.test(String(value))) callback()
        else callback(new Error('请输入正确的11位手机号'))
      }
    }
  ],
  employee: [{ required: true, message: '请选择服务员工', trigger: 'change' }],
  serviceType: [{ required: true, message: '请选择服务类型', trigger: 'change' }],
  projectId: [conditionalRequired(() => form.serviceType === '普通手工', '请选择预约服务项目')],
  packagePurchaseId: [
    conditionalRequired(() => form.serviceType === '套盒手工', '请选择会员可用套盒')
  ],
  startsAt: [{ required: true, message: '请选择预约时间', trigger: 'change' }],
  duration: [positiveNumberRule('请输入大于0的预计时长')]
}
const completionRules: FormRules<AppointmentCompletionInput> = {
  duration: [positiveNumberRule('请输入大于0的实际服务时长')],
  externalPaymentMethod: [
    {
      required: true,
      trigger: 'change',
      validator: (_rule, value, callback) => {
        if (!completionRequiresExternalPayment.value || value) callback()
        else callback(new Error('请选择实际支付方式'))
      }
    }
  ]
}

const availableMemberPackages = computed(() =>
  salonStore.packagePurchases.filter(
    item =>
      item.memberId === form.memberId &&
      item.packageType === '套盒' &&
      isPackagePurchaseAvailable(item, new Date(form.startsAt.replace(' ', 'T')))
  )
)
const selectedAppointmentProject = computed(() =>
  salonStore.projects.find(item => item.id === form.projectId)
)
const completingProject = computed(() =>
  salonStore.projects.find(item => item.id === completing.value?.projectId)
)
const completingMember = computed(() =>
  salonStore.members.find(item => item.id === completing.value?.memberId)
)
const completionBalancePayment = computed(() => {
  if (!completingProject.value || !completingMember.value) return 0
  return Math.min(completingProject.value.price, completingMember.value.balance)
})
const completionExternalPayment = computed(() =>
  completingProject.value
    ? Math.max(0, completingProject.value.price - completionBalancePayment.value)
    : 0
)
const completionRequiresExternalPayment = computed(
  () =>
    completing.value?.serviceType === '普通手工' &&
    (completing.value.customerType === 'guest' || completionExternalPayment.value > 0)
)
const filteredAppointments = computed(() =>
  salonStore.appointments.filter(item => {
    if (
      appliedFilters.customerName.trim() &&
      !item.customerName.includes(appliedFilters.customerName.trim())
    )
      return false
    if (
      appliedFilters.customerPhone.trim() &&
      !item.customerPhone.includes(appliedFilters.customerPhone.trim())
    )
      return false
    if (appliedFilters.employee && item.employee !== appliedFilters.employee) return false
    if (appliedFilters.serviceType && item.serviceType !== appliedFilters.serviceType) return false
    if (appliedFilters.status && item.status !== appliedFilters.status) return false
    if (appliedFilters.date && localDateKey(item.startsAt) !== appliedFilters.date) return false
    return true
  })
)
const {
  currentPage,
  pageSize,
  paginatedRecords: paginatedAppointments
} = useTablePagination(filteredAppointments)
const selectedCalendarDay = computed(() => localDateKey(calendarDate.value.toISOString()))
const selectedDayAppointments = computed(() =>
  salonStore.appointments
    .filter(item => localDateKey(item.startsAt) === selectedCalendarDay.value)
    .sort((a, b) => a.startsAt.localeCompare(b.startsAt))
)
const todayAppointments = computed(() =>
  salonStore.appointments.filter(item => localDateKey(item.startsAt) === localDateKey(new Date()))
)
const pendingToday = computed(
  () => todayAppointments.value.filter(item => item.status === 'pending').length
)
const activeToday = computed(
  () =>
    todayAppointments.value.filter(
      item => item.status === 'arrived' || item.status === 'in_service'
    ).length
)
const completedToday = computed(
  () => todayAppointments.value.filter(item => item.status === 'completed').length
)
const employeeAvailability = computed(() =>
  buildEmployeeAvailability(
    salonStore.employees,
    salonStore.appointments,
    availabilityDate.value,
    availabilityDuration.value
  )
)

function pad(value: number) {
  return String(value).padStart(2, '0')
}

function localDateTimeValue(value: Date | string) {
  const date = new Date(value)
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:00`
}

function toIso(value: string) {
  return new Date(value.replace(' ', 'T')).toISOString()
}

function statusMeta(status: AppointmentStatus) {
  return APPOINTMENT_STATUS_OPTIONS.find(item => item.value === status)!
}

function hasAppointmentActions(status: AppointmentStatus) {
  return status === 'pending' || status === 'arrived' || status === 'in_service'
}

function serviceTypeLabel(type: ServiceType) {
  return SERVICE_TYPE_LABELS[type]
}

function calendarDayAppointments(day: string) {
  return salonStore.appointments.filter(item => localDateKey(item.startsAt) === day)
}

function queryAppointments() {
  Object.assign(appliedFilters, filterForm)
}

function resetFilters() {
  Object.assign(filterForm, emptyFilters())
  Object.assign(appliedFilters, emptyFilters())
}

function disabledAvailabilityDate(date: Date) {
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  return date.getTime() < today.getTime()
}

function openAppointment(item?: Appointment) {
  editing.value = item ?? null
  if (item) {
    Object.assign(form, {
      customerType: item.customerType,
      memberId: item.memberId ?? '',
      guestName: item.customerType === 'guest' ? item.customerName : '',
      guestPhone: item.customerType === 'guest' ? item.customerPhone : '',
      employee: item.employee,
      serviceType: item.serviceType,
      projectId: item.projectId ?? '',
      packagePurchaseId: item.packagePurchaseId ?? '',
      startsAt: localDateTimeValue(item.startsAt),
      duration: item.duration,
      note: item.note
    })
  } else {
    Object.assign(form, {
      customerType: salonStore.members.length ? 'member' : 'guest',
      memberId: '',
      guestName: '',
      guestPhone: '',
      employee: '',
      serviceType: '普通手工',
      projectId: '',
      packagePurchaseId: '',
      startsAt: nextAppointmentTime(),
      duration: 60,
      note: ''
    })
  }
  modal.value = 'appointment'
}

function openAppointmentFromAvailability(employee: string, startsAt: string) {
  openAppointment()
  Object.assign(form, {
    employee,
    startsAt: localDateTimeValue(startsAt),
    duration: availabilityDuration.value
  })
}

async function submitAppointment() {
  if (!(await validateForm(formRef.value))) return
  saving.value = true
  try {
    const input: AppointmentInput = { ...form, startsAt: toIso(form.startsAt) }
    if (editing.value) await updateAppointment(editing.value.id, input)
    else await createAppointment(input)
    modal.value = null
    notify(editing.value ? '预约已更新' : '预约登记成功')
  } catch (reason) {
    notify(errorMessage(reason, '预约保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function changeStatus(item: Appointment, status: AppointmentStatus) {
  if (status === 'cancelled' || status === 'no_show') {
    try {
      await ElMessageBox.confirm(
        status === 'cancelled' ? '确认取消该预约吗？' : '确认将该预约标记为爽约吗？',
        status === 'cancelled' ? '取消预约' : '标记爽约',
        { type: 'warning', confirmButtonText: '确认', cancelButtonText: '返回' }
      )
    } catch {
      return
    }
  }
  try {
    await setAppointmentStatus(item.id, status)
    notify(statusMeta(status).label + '操作成功')
  } catch (reason) {
    notify(errorMessage(reason, '状态更新失败'), 'error')
  }
}

function openCompletion(item: Appointment) {
  completing.value = item
  Object.assign(completionForm, {
    appointmentId: item.id,
    duration: item.duration,
    externalPaymentMethod: ''
  })
  modal.value = 'complete'
}

async function submitCompletion() {
  if (!(await validateForm(completionFormRef.value)) || !completing.value) return
  if (completing.value.serviceType === '套盒手工') {
    try {
      await ElMessageBox.confirm(
        `确认完成“${completing.value.serviceName}”服务并扣减套盒1次吗？`,
        '确认完成服务',
        { type: 'warning', confirmButtonText: '确认完成', cancelButtonText: '取消' }
      )
    } catch {
      return
    }
  }
  saving.value = true
  try {
    await completeAppointment({ ...completionForm })
    modal.value = null
    notify('预约已完成，服务记录已生成')
  } catch (reason) {
    notify(errorMessage(reason, '完成服务失败'), 'error')
  } finally {
    saving.value = false
  }
}

watch(
  () => form.customerType,
  customerType => {
    if (customerType === 'guest' && form.serviceType === '套盒手工') form.serviceType = '普通手工'
    formRef.value?.clearValidate(['memberId', 'guestName', 'guestPhone'])
  }
)
watch(
  () => form.serviceType,
  serviceType => {
    if (serviceType === '套盒手工') form.customerType = 'member'
    else form.packagePurchaseId = ''
    formRef.value?.clearValidate(['projectId', 'packagePurchaseId'])
  }
)
watch(
  () => form.projectId,
  () => {
    if (form.serviceType === '普通手工' && selectedAppointmentProject.value)
      form.duration = selectedAppointmentProject.value.duration
  }
)
watch(
  () => form.memberId,
  () => {
    if (!editing.value || editing.value.memberId !== form.memberId) form.packagePurchaseId = ''
    formRef.value?.clearValidate('packagePurchaseId')
  }
)
watch(
  () => route.query.action,
  action => {
    if (action === 'new') openAppointment()
  },
  { immediate: true }
)
</script>

<template>
  <div class="page appointment-page">
    <section class="section-toolbar">
      <el-radio-group v-model="viewMode" class="salon-radio-group">
        <el-radio-button value="list">
          <span class="flex-horizontal-center gap-5">
            <List :size="15" />
            列表视图
          </span>
        </el-radio-button>
        <el-radio-button value="calendar">
          <span class="flex-horizontal-center gap-5">
            <CalendarDays :size="15" />
            日历视图
          </span>
        </el-radio-button>
        <el-radio-button value="availability">
          <span class="flex-horizontal-center gap-5">
            <Clock3 :size="15" />
            空闲时间
          </span>
        </el-radio-button>
      </el-radio-group>
      <el-button type="primary" @click="openAppointment()">
        <Plus :size="17" />
        登记预约
      </el-button>
    </section>

    <section class="summary-strip four">
      <div>
        <p>
          今日预约
          <strong>{{ todayAppointments.length }}</strong>
          <span>全部预约</span>
        </p>
      </div>
      <div>
        <p>
          待到店
          <strong>{{ pendingToday }}</strong>
          <span>等待顾客到店</span>
        </p>
      </div>
      <div>
        <p>
          进行中
          <strong>{{ activeToday }}</strong>
          <span>已到店或服务中</span>
        </p>
      </div>
      <div>
        <p>
          已完成
          <strong>{{ completedToday }}</strong>
          <span>已生成服务记录</span>
        </p>
      </div>
    </section>

    <section v-if="viewMode === 'list'" class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar appointment-filter-toolbar">
        <label class="table-filter-field">
          <span>顾客姓名</span>
          <el-input
            v-model="filterForm.customerName"
            clearable
            class="appointment-filter-control table-filter-input"
            placeholder="请输入顾客姓名"
            @keyup.enter="queryAppointments"
          />
        </label>
        <label class="table-filter-field">
          <span>手机号</span>
          <el-input
            v-model="filterForm.customerPhone"
            clearable
            class="appointment-filter-control table-filter-input"
            placeholder="请输入手机号"
            @keyup.enter="queryAppointments"
          />
        </label>
        <label class="table-filter-field">
          <span>服务员工</span>
          <el-select
            v-model="filterForm.employee"
            clearable
            class="appointment-filter-control table-filter-select"
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
          <span>服务类型</span>
          <el-select
            v-model="filterForm.serviceType"
            clearable
            class="appointment-filter-control table-filter-select"
            placeholder="请选择服务类型"
          >
            <el-option
              v-for="type in SERVICE_TYPES"
              :key="type"
              :label="serviceTypeLabel(type)"
              :value="type"
            />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>预约状态</span>
          <el-select
            v-model="filterForm.status"
            clearable
            class="appointment-filter-control table-filter-select"
            placeholder="请选择状态"
          >
            <el-option
              v-for="item in APPOINTMENT_STATUS_OPTIONS"
              :key="item.value"
              :label="item.label"
              :value="item.value"
            />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>预约日期</span>
          <el-date-picker
            v-model="filterForm.date"
            class="appointment-filter-date service-date-picker"
            type="date"
            format="YYYY年MM月DD日"
            value-format="YYYY-MM-DD"
            clearable
            placeholder="请选择日期"
          />
        </label>
        <div class="service-filter-actions">
          <el-button type="primary" @click="queryAppointments">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetFilters">重置</el-button>
        </div>
      </div>

      <el-table :data="paginatedAppointments" row-key="id" stripe class="salon-table">
        <el-table-column label="预约时间" min-width="155">
          <template #default="{ row }">{{ fullDateTime(row.startsAt) }}</template>
        </el-table-column>
        <el-table-column label="顾客信息" min-width="145">
          <template #default="{ row }">
            <div>{{ row.customerName }}</div>
            <div class="service-customer-phone">{{ row.customerPhone }}</div>
          </template>
        </el-table-column>
        <el-table-column prop="employee" label="服务员工" min-width="100" />
        <el-table-column label="服务类型" min-width="100">
          <template #default="{ row }">
            <el-tag round :type="row.serviceType === '套盒手工' ? 'warning' : 'info'">
              {{ serviceTypeLabel(row.serviceType) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="serviceName" label="服务项目/套盒" min-width="145" />
        <el-table-column label="预计时长" min-width="90">
          <template #default="{ row }">{{ row.duration }}分钟</template>
        </el-table-column>
        <el-table-column label="状态" min-width="90">
          <template #default="{ row }">
            <el-tag round :type="statusMeta(row.status).type">
              {{ statusMeta(row.status).label }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="note" label="备注" min-width="130" show-overflow-tooltip />
        <el-table-column label="登记信息" min-width="155">
          <template #default="{ row }">
            <div>{{ row.createdBy }}</div>
            <div class="service-customer-phone">{{ fullDateTime(row.createdAt) }}</div>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="{ row }">
            <div
              v-if="hasAppointmentActions(row.status)"
              class="element-row-actions appointment-actions"
            >
              <el-button
                v-if="row.status === 'pending'"
                size="small"
                type="primary"
                @click="changeStatus(row as Appointment, 'arrived')"
              >
                标记到店
              </el-button>
              <el-button
                v-if="row.status === 'arrived'"
                size="small"
                type="primary"
                @click="changeStatus(row as Appointment, 'in_service')"
              >
                开始服务
              </el-button>
              <el-button
                v-if="row.status === 'in_service'"
                size="small"
                type="success"
                @click="openCompletion(row as Appointment)"
              >
                完成服务
              </el-button>
              <el-button
                v-if="row.status === 'pending' || row.status === 'arrived'"
                size="small"
                @click="openAppointment(row as Appointment)"
              >
                编辑
              </el-button>
              <el-button
                v-if="row.status === 'pending' || row.status === 'arrived'"
                size="small"
                @click="changeStatus(row as Appointment, 'cancelled')"
              >
                取消
              </el-button>
              <el-button
                v-if="row.status === 'pending'"
                size="small"
                type="danger"
                plain
                @click="changeStatus(row as Appointment, 'no_show')"
              >
                爽约
              </el-button>
            </div>
            <span v-else class="appointment-empty-action">--</span>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="currentPage"
        v-model:page-size="pageSize"
        :total="filteredAppointments.length"
      />
    </section>

    <section v-else-if="viewMode === 'calendar'" class="appointment-calendar-grid">
      <article class="panel appointment-calendar-panel">
        <el-calendar v-model="calendarDate">
          <template #date-cell="{ data }">
            <div class="appointment-calendar-cell">
              <span>{{ Number(data.day.slice(-2)) }}</span>
              <div class="appointment-calendar-dots">
                <i
                  v-for="item in calendarDayAppointments(data.day).slice(0, 4)"
                  :key="item.id"
                  :class="`status-${item.status}`"
                />
              </div>
              <small v-if="calendarDayAppointments(data.day).length">
                {{ calendarDayAppointments(data.day).length }}项
              </small>
            </div>
          </template>
        </el-calendar>
      </article>
      <article class="panel appointment-day-panel">
        <header class="panel-head">
          <div>
            <h3>{{ selectedCalendarDay }} 预约</h3>
            <p>按预约时间顺序展示</p>
          </div>
        </header>
        <div v-if="selectedDayAppointments.length" class="appointment-day-list">
          <button
            v-for="item in selectedDayAppointments"
            :key="item.id"
            type="button"
            class="appointment-day-item"
            @click="
              item.status === 'pending' || item.status === 'arrived'
                ? openAppointment(item)
                : undefined
            "
          >
            <time>{{ localDateTimeValue(item.startsAt).slice(11, 16) }}</time>
            <div>
              <b>{{ item.customerName }} · {{ item.serviceName }}</b>
              <span>{{ item.employee }} · {{ item.duration }}分钟</span>
            </div>
            <el-tag round size="small" :type="statusMeta(item.status).type">
              {{ statusMeta(item.status).label }}
            </el-tag>
          </button>
        </div>
        <div v-else class="empty-state compact">
          <CalendarDays :size="27" />
          <b>当天暂无预约</b>
          <span>可点击“登记预约”新增安排</span>
        </div>
      </article>
    </section>

    <section v-else class="panel appointment-availability-panel">
      <div class="appointment-availability-toolbar">
        <div>
          <h3>员工空闲时间</h3>
          <p>
            按 {{ APPOINTMENT_BUSINESS_HOURS.start }}–{{ APPOINTMENT_BUSINESS_HOURS.end }}
            营业时间计算，取消和爽约不占用时间
          </p>
        </div>
        <div class="appointment-availability-filters">
          <label class="table-filter-field">
            <span>查看日期</span>
            <el-date-picker
              v-model="availabilityDate"
              type="date"
              format="YYYY年MM月DD日"
              value-format="YYYY-MM-DD"
              :disabled-date="disabledAvailabilityDate"
              :clearable="false"
            />
          </label>
          <label class="table-filter-field">
            <span>预计服务时长</span>
            <el-select v-model="availabilityDuration" class="appointment-duration-select">
              <el-option :value="30" label="30分钟" />
              <el-option :value="60" label="60分钟" />
              <el-option :value="90" label="90分钟" />
              <el-option :value="120" label="120分钟" />
              <el-option :value="180" label="180分钟" />
            </el-select>
          </label>
        </div>
      </div>
      <div class="appointment-availability-grid">
        <article
          v-for="employee in employeeAvailability"
          :key="employee.employeeId"
          class="appointment-employee-card"
        >
          <header>
            <span class="appointment-employee-avatar" :style="{ background: employee.color }">
              {{ employee.employeeName.slice(-1) }}
            </span>
            <div>
              <h4>{{ employee.employeeName }}</h4>
              <p>可用 {{ employee.freeMinutes }} 分钟 · 已占用 {{ employee.busyMinutes }} 分钟</p>
            </div>
          </header>
          <div v-if="employee.slots.length" class="appointment-slot-list">
            <el-button
              v-for="slot in employee.slots"
              :key="slot.startsAt"
              class="appointment-slot-button"
              @click="openAppointmentFromAvailability(employee.employeeName, slot.startsAt)"
            >
              <Clock3 :size="15" />
              <span>{{ slot.startLabel }}–{{ slot.endLabel }}</span>
              <small>{{ slot.duration }}分钟</small>
            </el-button>
          </div>
          <div v-else class="appointment-no-slot">
            <Clock3 :size="21" />
            <span>没有满足 {{ availabilityDuration }} 分钟的空闲时间</span>
          </div>
        </article>
      </div>
    </section>

    <BaseModal
      v-if="modal === 'appointment'"
      :title="editing ? '编辑预约' : '登记预约'"
      subtitle="预约不会提前扣款或扣减套盒次数"
      wide
      @close="modal = null"
    >
      <el-form ref="formRef" :model="form" :rules="rules" scroll-to-error label-position="top">
        <div class="form-grid">
          <el-form-item label="顾客类型" prop="customerType">
            <el-radio-group v-model="form.customerType">
              <el-radio-button value="member">会员</el-radio-button>
              <el-radio-button value="guest">游客</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="服务类型" prop="serviceType">
            <el-radio-group v-model="form.serviceType">
              <el-radio-button value="普通手工">普通消费</el-radio-button>
              <el-radio-button value="套盒手工">套盒消费</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item
            v-if="form.customerType === 'member'"
            class="span-2"
            label="预约会员"
            prop="memberId"
          >
            <MemberSelect v-model="form.memberId" />
          </el-form-item>
          <template v-else>
            <el-form-item label="游客姓名" prop="guestName">
              <el-input v-model="form.guestName" clearable placeholder="例如：王女士" />
            </el-form-item>
            <el-form-item label="游客手机号" prop="guestPhone">
              <el-input
                v-model="form.guestPhone"
                clearable
                maxlength="11"
                placeholder="请输入手机号"
              />
            </el-form-item>
          </template>
          <el-form-item label="服务员工" prop="employee">
            <EmployeeSelect v-model="form.employee" />
          </el-form-item>
          <el-form-item label="预约时间" prop="startsAt">
            <el-date-picker
              v-model="form.startsAt"
              type="datetime"
              format="YYYY-MM-DD HH:mm"
              value-format="YYYY-MM-DD HH:mm:ss"
              placeholder="请选择预约时间"
            />
          </el-form-item>
          <el-form-item
            v-if="form.serviceType === '普通手工'"
            class="span-2"
            label="服务项目"
            prop="projectId"
          >
            <el-select v-model="form.projectId" filterable clearable placeholder="请选择服务项目">
              <el-option
                v-for="item in salonStore.projects.filter(project => project.status === 'active')"
                :key="item.id"
                :label="`${item.name} · ${currency(item.price)} · ${item.duration}分钟`"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
          <el-form-item v-else class="span-2" label="会员套盒" prop="packagePurchaseId">
            <el-select
              v-model="form.packagePurchaseId"
              :disabled="!form.memberId"
              clearable
              placeholder="请选择会员可用套盒"
            >
              <el-option
                v-for="item in availableMemberPackages"
                :key="item.id"
                :label="`${item.packageName}（${packagePurchaseLimitText(item)}）`"
                :value="item.id"
              />
            </el-select>
            <span v-if="form.memberId && !availableMemberPackages.length" class="field-help">
              该会员暂无可用套盒
            </span>
          </el-form-item>
          <el-form-item label="预计时长（分钟）" prop="duration">
            <el-input-number
              align="left"
              v-model="form.duration"
              :min="1"
              :max="1440"
              :controls="false"
              :disabled="form.serviceType === '普通手工'"
            />
          </el-form-item>
          <el-form-item class="span-2" label="备注" prop="note">
            <el-input
              v-model="form.note"
              type="textarea"
              :rows="3"
              maxlength="200"
              show-word-limit
              placeholder="可填写顾客需求或注意事项"
            />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitAppointment">保存预约</el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="modal === 'complete' && completing"
      title="完成预约服务"
      :subtitle="`${completing.customerName} · ${completing.serviceName}`"
      @close="modal = null"
    >
      <el-form
        ref="completionFormRef"
        :model="completionForm"
        :rules="completionRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item label="服务时长（分钟）" prop="duration">
            <el-input-number
              v-model="completionForm.duration"
              :min="1"
              :controls="false"
              :disabled="completing.serviceType === '普通手工'"
            />
          </el-form-item>
          <el-form-item
            v-if="completionRequiresExternalPayment"
            label="实际支付方式"
            prop="externalPaymentMethod"
          >
            <el-select v-model="completionForm.externalPaymentMethod">
              <el-option
                v-for="item in RECHARGE_PAYMENT_METHODS"
                :key="item"
                :label="item"
                :value="item"
              />
            </el-select>
          </el-form-item>
        </div>
        <div class="form-tip">
          {{
            completing.serviceType === '套盒手工'
              ? '确认后将扣减对应套盒1次，并生成套盒消费记录。'
              : `项目金额 ${currency(completingProject?.price ?? 0)}，余额支付 ${currency(completionBalancePayment)}，实际支付 ${currency(completionExternalPayment)}。确认后自动扣款并计入消费营业额。`
          }}
        </div>
      </el-form>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitCompletion">确认完成</el-button>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.appointment-filter-toolbar {
  align-items: flex-end;
  flex-wrap: wrap;
  row-gap: 12px;
}
.appointment-filter-toolbar .table-filter-field {
  flex: 0 0 auto;
}
.appointment-filter-toolbar .appointment-filter-control {
  width: 180px;
}
.appointment-filter-toolbar .appointment-filter-date {
  width: 190px;
}
.appointment-actions {
  flex-wrap: wrap;
}
.appointment-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
.appointment-empty-action {
  display: block;
  color: var(--muted);
  text-align: center;
}
.appointment-calendar-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.55fr) minmax(300px, 0.65fr);
  gap: 18px;
}
.appointment-calendar-panel {
  overflow: hidden;
}
.appointment-calendar-panel :deep(.el-calendar) {
  --el-calendar-cell-width: 60px;
}
.appointment-calendar-panel :deep(.el-calendar-day) {
  height: 82px;
  padding: 7px;
}
.appointment-calendar-cell {
  height: 100%;
  position: relative;
}
.appointment-calendar-cell > span {
  font-weight: 650;
}
.appointment-calendar-cell small {
  position: absolute;
  right: 0;
  bottom: 0;
  color: var(--muted);
  font-size: 12px;
}
.appointment-calendar-dots {
  display: flex;
  gap: 3px;
  margin-top: 8px;
}
.appointment-calendar-dots i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #7b8fc8;
}
.appointment-calendar-dots .status-arrived,
.appointment-calendar-dots .status-in_service {
  background: #e3a650;
}
.appointment-calendar-dots .status-completed {
  background: #67a98e;
}
.appointment-calendar-dots .status-cancelled {
  background: #a8a8ad;
}
.appointment-calendar-dots .status-no_show {
  background: #d35e67;
}
.appointment-day-panel {
  padding: 20px;
  min-height: 520px;
}
.appointment-day-list {
  display: grid;
  gap: 10px;
}
.appointment-day-item {
  width: 100%;
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: white;
  text-align: left;
  cursor: pointer;
}
.appointment-day-item:hover {
  border-color: var(--rose);
}
.appointment-day-item time {
  color: var(--rose);
  font-weight: 750;
}
.appointment-day-item div {
  display: grid;
  gap: 4px;
  min-width: 0;
}
.appointment-day-item b,
.appointment-day-item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.appointment-day-item span {
  color: var(--muted);
  font-size: 12px;
}
.appointment-availability-panel {
  padding: 20px;
}
.appointment-availability-toolbar {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
  margin-bottom: 18px;
}
.appointment-availability-toolbar h3,
.appointment-employee-card h4 {
  margin: 0;
}
.appointment-availability-toolbar p,
.appointment-employee-card p {
  margin: 5px 0 0;
  color: var(--muted);
  font-size: 12px;
}
.appointment-availability-filters {
  display: flex;
  align-items: center;
  gap: 14px;
}
.appointment-duration-select {
  width: 120px;
}
.appointment-availability-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}
.appointment-employee-card {
  min-height: 180px;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 13px;
  background: #fcfaf9;
}
.appointment-employee-card > header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
}
.appointment-employee-avatar {
  width: 38px;
  height: 38px;
  display: grid;
  place-items: center;
  flex: 0 0 auto;
  border-radius: 11px;
  color: white;
  font-weight: 700;
}
.appointment-slot-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.appointment-slot-button {
  height: auto;
  margin: 0;
  padding: 9px 11px;
}
.appointment-slot-button span {
  font-weight: 650;
}
.appointment-slot-button small {
  color: var(--muted);
}
.appointment-no-slot {
  min-height: 80px;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 7px;
  color: var(--muted);
  font-size: 12px;
}
@media (max-width: 1100px) {
  .appointment-calendar-grid {
    grid-template-columns: 1fr;
  }
  .appointment-availability-grid {
    grid-template-columns: 1fr;
  }
  .appointment-availability-toolbar {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
