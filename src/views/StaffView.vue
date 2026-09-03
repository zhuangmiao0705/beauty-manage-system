<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, Search, ShieldCheck, UserCog, UsersRound } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import AccountTable from '../components/staff/AccountTable.vue'
import EmployeeTable from '../components/staff/EmployeeTable.vue'
import { DEFAULT_EMPLOYEE_COMPENSATION, EMPLOYEE_ROLES } from '../config/options'
import { createAccount, getAccounts, resetAccountPassword, setAccountStatus } from '../auth'
import { addEmployee, salonStore, setEmployeeStatus } from '../data/repository'
import type { AccountInput, AccountRecord, Employee, EmployeeInput } from '../types'
import { errorMessage, notify } from '../utils/feedback'
import { requiredTextRule, validateForm } from '../utils/validation'

type EmployeeForm = Omit<EmployeeInput, 'baseCommissionRate' | 'excessCommissionRate'> & {
  baseCommissionPercent: number
  excessCommissionPercent: number
}

const activeTab = ref<'employees' | 'accounts'>('employees')
const employeeFilters = reactive({ name: '', role: '' })
const accountFilters = reactive({ username: '', displayName: '' })
const modal = ref<'employee' | 'account' | 'reset' | null>(null)
const saving = ref(false)
const employeeFormRef = ref<FormInstance>()
const accountFormRef = ref<FormInstance>()
const resetFormRef = ref<FormInstance>()
const accounts = ref<AccountRecord[]>([])
const selectedAccount = ref<AccountRecord | null>(null)
const employeeForm = reactive<EmployeeForm>({
  name: '',
  role: '美容师',
  color: '#cc6278',
  baseSalary: DEFAULT_EMPLOYEE_COMPENSATION.baseSalary,
  baseCommissionPercent: DEFAULT_EMPLOYEE_COMPENSATION.baseCommissionRate * 100,
  performanceTarget: DEFAULT_EMPLOYEE_COMPENSATION.performanceTarget,
  excessCommissionPercent: DEFAULT_EMPLOYEE_COMPENSATION.excessCommissionRate * 100,
  mealAllowancePerDay: DEFAULT_EMPLOYEE_COMPENSATION.mealAllowancePerDay,
  attendanceBonus: DEFAULT_EMPLOYEE_COMPENSATION.attendanceBonus,
  normalServiceCommission: DEFAULT_EMPLOYEE_COMPENSATION.normalServiceCommission,
  packageServiceCommission: DEFAULT_EMPLOYEE_COMPENSATION.packageServiceCommission
})
const accountForm = reactive<AccountInput>({
  username: '',
  displayName: '',
  role: 'employee',
  employeeId: null,
  password: '123456'
})
const resetForm = reactive({ password: '123456' })
const nonNegativeRule = (message: string, max?: number) => ({
  required: true,
  type: 'number' as const,
  min: 0,
  ...(max === undefined ? {} : { max }),
  message,
  trigger: ['blur', 'change']
})
const employeeRules: FormRules<EmployeeForm> = {
  name: [requiredTextRule('请输入员工姓名')],
  role: [{ required: true, message: '请选择岗位', trigger: 'change' }],
  color: [{ required: true, message: '请选择标识颜色', trigger: 'change' }],
  baseSalary: [nonNegativeRule('请输入有效的员工底薪')],
  baseCommissionPercent: [nonNegativeRule('请输入0至100的基础提成比例', 100)],
  performanceTarget: [nonNegativeRule('请输入有效的目标绩效')],
  excessCommissionPercent: [nonNegativeRule('请输入0至100的超额提成比例', 100)],
  mealAllowancePerDay: [nonNegativeRule('请输入有效的每日餐补')],
  attendanceBonus: [nonNegativeRule('请输入有效的全勤奖')],
  normalServiceCommission: [nonNegativeRule('请输入有效的普通手工提成')],
  packageServiceCommission: [nonNegativeRule('请输入有效的套盒手工提成')]
}
const accountRules: FormRules<AccountInput> = {
  username: [requiredTextRule('请输入登录账号')],
  role: [{ required: true, message: '请选择账号角色', trigger: 'change' }],
  displayName: [requiredTextRule('请输入显示姓名')],
  password: [
    { required: true, message: '请输入初始密码', trigger: ['blur', 'change'] },
    { min: 6, message: '初始密码至少需要 6 位', trigger: ['blur', 'change'] }
  ]
}
const resetRules: FormRules<typeof resetForm> = {
  password: [
    { required: true, message: '请输入新密码', trigger: ['blur', 'change'] },
    { min: 6, message: '新密码至少需要 6 位', trigger: ['blur', 'change'] }
  ]
}

const filteredEmployees = computed(() =>
  salonStore.employees.filter(item => {
    const name = employeeFilters.name.trim()
    return (
      (!name || item.name.includes(name)) &&
      (!employeeFilters.role || item.role === employeeFilters.role)
    )
  })
)
const filteredAccounts = computed(() =>
  accounts.value.filter(item => {
    const username = accountFilters.username.trim()
    const displayName = accountFilters.displayName.trim()
    return (
      (!username || item.username.includes(username)) &&
      (!displayName || item.displayName.includes(displayName))
    )
  })
)

onMounted(refreshAccounts)

async function refreshAccounts() {
  try {
    accounts.value = [...(await getAccounts())]
  } catch (reason) {
    notify(errorMessage(reason, '账号加载失败'), 'error')
  }
}

function openEmployee() {
  Object.assign(employeeForm, {
    name: '',
    role: '美容师',
    color: '#cc6278',
    baseSalary: DEFAULT_EMPLOYEE_COMPENSATION.baseSalary,
    baseCommissionPercent: DEFAULT_EMPLOYEE_COMPENSATION.baseCommissionRate * 100,
    performanceTarget: DEFAULT_EMPLOYEE_COMPENSATION.performanceTarget,
    excessCommissionPercent: DEFAULT_EMPLOYEE_COMPENSATION.excessCommissionRate * 100,
    mealAllowancePerDay: DEFAULT_EMPLOYEE_COMPENSATION.mealAllowancePerDay,
    attendanceBonus: DEFAULT_EMPLOYEE_COMPENSATION.attendanceBonus,
    normalServiceCommission: DEFAULT_EMPLOYEE_COMPENSATION.normalServiceCommission,
    packageServiceCommission: DEFAULT_EMPLOYEE_COMPENSATION.packageServiceCommission
  })
  modal.value = 'employee'
}

function openAccount() {
  Object.assign(accountForm, {
    username: '',
    displayName: '员工账号',
    role: 'employee',
    employeeId: null,
    password: '123456'
  })
  modal.value = 'account'
}

async function submitEmployee() {
  if (!(await validateForm(employeeFormRef.value))) return
  saving.value = true
  try {
    const { baseCommissionPercent, excessCommissionPercent, ...profile } = employeeForm
    await addEmployee({
      ...profile,
      baseCommissionRate: baseCommissionPercent / 100,
      excessCommissionRate: excessCommissionPercent / 100
    })
    modal.value = null
    notify('员工档案创建成功')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function submitAccount() {
  if (!(await validateForm(accountFormRef.value))) return
  saving.value = true
  try {
    accounts.value = [...(await createAccount({ ...accountForm }))]
    modal.value = null
    notify('登录账号创建成功')
  } catch (reason) {
    notify(errorMessage(reason, '账号创建失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function toggleEmployee(employee: Employee) {
  const status = employee.status === 'active' ? 'inactive' : 'active'
  try {
    await setEmployeeStatus(employee.id, status)
    notify(status === 'active' ? '员工已启用' : '员工已停用')
  } catch (reason) {
    notify(errorMessage(reason, '操作失败'), 'error')
  }
}

async function toggleAccount(account: AccountRecord) {
  const status = account.status === 'active' ? 'inactive' : 'active'
  try {
    accounts.value = [...(await setAccountStatus(account.id, status))]
    notify(status === 'active' ? '账号已启用' : '账号已停用')
  } catch (reason) {
    notify(errorMessage(reason, '操作失败'), 'error')
  }
}

function openReset(account: AccountRecord) {
  selectedAccount.value = account
  resetForm.password = '123456'
  modal.value = 'reset'
}
async function submitReset() {
  if (!selectedAccount.value || !(await validateForm(resetFormRef.value))) return
  saving.value = true
  try {
    accounts.value = [...(await resetAccountPassword(selectedAccount.value.id, resetForm.password))]
    modal.value = null
    notify('密码已重置，新密码可直接用于登录')
  } catch (reason) {
    notify(errorMessage(reason, '密码重置失败'), 'error')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="page staff-page">
    <section class="section-toolbar">
      <el-radio-group v-model="activeTab" class="salon-radio-group">
        <el-radio-button value="employees">员工档案</el-radio-button>
        <el-radio-button value="accounts">登录账号</el-radio-button>
      </el-radio-group>
      <el-button v-if="activeTab === 'employees'" type="primary" @click="openEmployee">
        <Plus :size="17" />
        新增员工
      </el-button>
      <el-button v-else type="primary" @click="openAccount">
        <Plus :size="17" />
        新增账号
      </el-button>
    </section>
    <section class="summary-strip">
      <div>
        <span class="summary-icon rose"><UsersRound :size="20" /></span>
        <p>
          员工总数
          <strong>{{ salonStore.employees.length }}</strong>
          <span>在职与停用档案</span>
        </p>
      </div>
      <div>
        <span class="summary-icon green"><UserCog :size="20" /></span>
        <p>
          启用账号
          <strong>{{ accounts.filter(item => item.status === 'active').length }}</strong>
          <span>可以正常登录</span>
        </p>
      </div>
      <div>
        <span class="summary-icon violet"><ShieldCheck :size="20" /></span>
        <p>
          店长账号
          <strong>{{ accounts.filter(item => item.role === 'manager').length }}</strong>
          <span>拥有全部权限</span>
        </p>
      </div>
    </section>
    <section class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <template v-if="activeTab === 'employees'">
          <label class="table-filter-field">
            <span>员工姓名</span>
            <el-input
              v-model="employeeFilters.name"
              clearable
              class="table-filter-input"
              placeholder="请输入员工姓名"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
          <label class="table-filter-field">
            <span>岗位</span>
            <el-select
              v-model="employeeFilters.role"
              clearable
              class="table-filter-select"
              placeholder="请选择岗位"
            >
              <el-option v-for="role in EMPLOYEE_ROLES" :key="role" :label="role" :value="role" />
            </el-select>
          </label>
        </template>
        <template v-else>
          <label class="table-filter-field">
            <span>登录账号</span>
            <el-input
              v-model="accountFilters.username"
              clearable
              class="table-filter-input"
              placeholder="请输入登录账号"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
          <label class="table-filter-field">
            <span>使用人</span>
            <el-input
              v-model="accountFilters.displayName"
              clearable
              class="table-filter-input"
              placeholder="请输入使用人姓名"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
        </template>
      </div>
      <EmployeeTable
        v-if="activeTab === 'employees'"
        :employees="filteredEmployees"
        @toggle="toggleEmployee"
      />
      <AccountTable
        v-else
        :accounts="filteredAccounts"
        @reset="openReset"
        @toggle="toggleAccount"
      />
    </section>
    <BaseModal
      v-if="modal === 'employee'"
      title="新增员工"
      subtitle="创建员工档案并设置独立薪酬规则"
      wide
      @close="modal = null"
    >
      <el-form
        ref="employeeFormRef"
        :model="employeeForm"
        :rules="employeeRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item label="员工姓名" prop="name">
            <el-input v-model="employeeForm.name" clearable />
          </el-form-item>
          <el-form-item label="岗位" prop="role">
            <el-select v-model="employeeForm.role">
              <el-option v-for="item in EMPLOYEE_ROLES" :key="item" :label="item" :value="item" />
            </el-select>
          </el-form-item>
          <el-form-item label="标识颜色" prop="color">
            <el-color-picker v-model="employeeForm.color" />
          </el-form-item>
          <el-form-item label="员工底薪" prop="baseSalary">
            <el-input-number
              v-model="employeeForm.baseSalary"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="员工基础提成（%）" prop="baseCommissionPercent">
            <el-input-number
              v-model="employeeForm.baseCommissionPercent"
              :min="0"
              :max="100"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="员工目标绩效" prop="performanceTarget">
            <el-input-number
              v-model="employeeForm.performanceTarget"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="超出目标绩效提成（%）" prop="excessCommissionPercent">
            <el-input-number
              v-model="employeeForm.excessCommissionPercent"
              :min="0"
              :max="100"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="餐补金额（元/天）" prop="mealAllowancePerDay">
            <el-input-number
              v-model="employeeForm.mealAllowancePerDay"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="全勤奖" prop="attendanceBonus">
            <el-input-number
              v-model="employeeForm.attendanceBonus"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="普通手工提成（元/次）" prop="normalServiceCommission">
            <el-input-number
              v-model="employeeForm.normalServiceCommission"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="套盒手工提成（元/次）" prop="packageServiceCommission">
            <el-input-number
              v-model="employeeForm.packageServiceCommission"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
        </div>
      </el-form>
      <div class="form-tip">
        绩效总金额 = 会员充值 + 套餐额外支付；会员本金支付部分不计销售员业绩。提成 = 绩效基础提成 +
        超目标提成 + 两类手工次数提成。
      </div>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitEmployee">保存员工</el-button>
      </template>
    </BaseModal>
    <BaseModal
      v-if="modal === 'account'"
      title="新增登录账号"
      subtitle="员工可共用账号登录，业务归属在登记时选择"
      @close="modal = null"
    >
      <el-form
        ref="accountFormRef"
        :model="accountForm"
        :rules="accountRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item label="登录账号" prop="username">
            <el-input v-model="accountForm.username" clearable autocomplete="off" />
          </el-form-item>
          <el-form-item label="账号角色" prop="role">
            <el-select v-model="accountForm.role">
              <el-option label="员工" value="employee" />
              <el-option label="店长" value="manager" />
            </el-select>
          </el-form-item>
          <el-form-item label="显示姓名" prop="displayName">
            <el-input v-model="accountForm.displayName" clearable />
          </el-form-item>
          <el-form-item class="span-2" label="初始密码" prop="password">
            <el-input
              v-model="accountForm.password"
              type="password"
              show-password
              autocomplete="new-password"
            />
            <span class="field-help">员工可在个人账号中自行修改密码</span>
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitAccount">创建账号</el-button>
      </template>
    </BaseModal>
    <BaseModal
      v-if="modal === 'reset' && selectedAccount"
      title="重置登录密码"
      :subtitle="`${selectedAccount.displayName} · ${selectedAccount.username}`"
      @close="modal = null"
    >
      <el-alert title="重置后，新密码可直接用于登录。" type="warning" :closable="false" show-icon />
      <el-form
        ref="resetFormRef"
        :model="resetForm"
        :rules="resetRules"
        scroll-to-error
        label-position="top"
        class="reset-password-form"
      >
        <el-form-item label="新初始密码" prop="password">
          <el-input
            v-model="resetForm.password"
            type="password"
            show-password
            autocomplete="new-password"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitReset">确认重置</el-button>
      </template>
    </BaseModal>
  </div>
</template>
