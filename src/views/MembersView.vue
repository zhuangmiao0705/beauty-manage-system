<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import 'element-plus/es/components/message-box/style/css'
import type { FormInstance, FormItemRule, FormRules } from 'element-plus'
import { useRoute, useRouter } from 'vue-router'
import {
  CircleDollarSign,
  Eye,
  Plus,
  RotateCcw,
  Search,
  UserPlus,
  WalletCards
} from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import EmployeeSelect from '../components/EmployeeSelect.vue'
import MemberSelect from '../components/MemberSelect.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { RECHARGE_PAYMENT_METHODS } from '../config/options'
import { authStore } from '../auth'
import { addMember, addTransaction, refundMemberAccount, salonStore } from '../data/repository'
import type { Member, MemberInput, TransactionInput } from '../types'
import { currency, dateOnly, dateTime, fullDateTime } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { positiveNumberRule, requiredTextRule, validateForm } from '../utils/validation'

const route = useRoute()
const router = useRouter()
const activeTab = ref(
  route.query.tab === 'transactions'
    ? 'transactions'
    : route.query.tab === 'refunds'
      ? 'refunds'
      : 'members'
)
const memberFilters = reactive({ name: '', phone: '' })
const transactionFilters = reactive({ memberName: '', item: '', note: '', employee: '' })
const modal = ref<'member' | 'transaction' | 'detail' | 'accountRefund' | null>(null)
const selectedMember = ref<Member | null>(null)
const saving = ref(false)
const memberFormRef = ref<FormInstance>()
const transactionFormRef = ref<FormInstance>()
const accountRefundFormRef = ref<FormInstance>()
const memberForm = reactive<MemberInput>({
  name: '',
  phone: '',
  initialBalance: 0,
  giftAmount: 0,
  employee: ''
})
const transactionForm = reactive<TransactionInput>({
  memberId: '',
  type: 'recharge',
  amount: 0,
  giftAmount: 0,
  paymentMethod: '微信支付',
  item: '会员充值',
  employee: '',
  note: ''
})
const accountRefundForm = reactive({ amount: 0, employee: '', note: '' })
const memberGiftRule: FormItemRule = {
  trigger: 'change',
  validator: (_rule, value, callback) => {
    if (Number(value) <= 0 || memberForm.initialBalance > 0) callback()
    else callback(new Error('填写赠送金额时，开卡充值金额必须大于0'))
  }
}
const memberRules: FormRules<MemberInput> = {
  name: [requiredTextRule('请输入会员姓名')],
  phone: [requiredTextRule('请输入手机号码')],
  initialBalance: [
    { type: 'number', min: 0, message: '开卡充值金额不能小于 0', trigger: ['blur', 'change'] }
  ],
  giftAmount: [
    { type: 'number', min: 0, message: '赠送金额不能小于 0', trigger: ['blur', 'change'] },
    memberGiftRule
  ],
  employee: [{ required: true, message: '请选择经办员工', trigger: 'change' }]
}
const transactionRules: FormRules<TransactionInput> = {
  memberId: [{ required: true, message: '请选择会员', trigger: 'change' }],
  amount: [positiveNumberRule('请输入大于 0 的金额')],
  giftAmount: [
    { type: 'number', min: 0, message: '赠送金额不能小于 0', trigger: ['blur', 'change'] }
  ],
  paymentMethod: [{ required: true, message: '请选择支付方式', trigger: 'change' }],
  employee: [{ required: true, message: '请选择经办员工', trigger: 'change' }]
}
const accountRefundRules = computed<FormRules<typeof accountRefundForm>>(() => ({
  amount: [
    {
      required: true,
      trigger: ['blur', 'change'],
      validator: (_rule, value, callback) => {
        const amount = Number(value)
        const available = selectedMember.value?.refundablePrincipal ?? 0
        if (!Number.isFinite(amount) || amount <= 0) callback(new Error('请输入大于0的退款金额'))
        else if (amount > available + 0.001)
          callback(new Error(`退款金额不能超过${currency(available)}`))
        else callback()
      }
    }
  ],
  employee: [{ required: true, message: '请选择绩效归属员工', trigger: 'change' }]
}))
const accountRefundIsFull = computed(
  () =>
    !!selectedMember.value &&
    accountRefundForm.amount >= selectedMember.value.refundablePrincipal - 0.001
)

const filteredMembers = computed(() =>
  salonStore.members.filter(member => {
    const name = memberFilters.name.trim()
    const phone = memberFilters.phone.trim()
    return (!name || member.name.includes(name)) && (!phone || member.phone.includes(phone))
  })
)
const filteredTransactions = computed(() =>
  salonStore.transactions.filter(item => {
    const memberName = transactionFilters.memberName.trim()
    const transactionItem = transactionFilters.item.trim()
    const note = transactionFilters.note.trim()
    const employee = transactionFilters.employee.trim()
    return (
      (!memberName || item.memberName.includes(memberName)) &&
      (!transactionItem || item.item.includes(transactionItem)) &&
      (!note || item.note.includes(note)) &&
      (!employee || item.employee.includes(employee))
    )
  })
)
const filteredRefunds = computed(() =>
  salonStore.refundRecords.filter(item => {
    const memberName = transactionFilters.memberName.trim()
    const transactionItem = transactionFilters.item.trim()
    const note = transactionFilters.note.trim()
    const employee = transactionFilters.employee.trim()
    const itemName = item.refundType === 'account' ? '账户退款' : '套餐退款'
    return (
      (!memberName || item.memberName.includes(memberName)) &&
      (!transactionItem || itemName.includes(transactionItem)) &&
      (!note || item.note.includes(note)) &&
      (!employee || item.employee.includes(employee) || item.operator.includes(employee))
    )
  })
)
const memberPhoneById = computed(
  () => new Map(salonStore.members.map(member => [member.id, member.phone]))
)
const {
  currentPage: memberPage,
  pageSize: memberPageSize,
  paginatedRecords: paginatedMembers
} = useTablePagination(filteredMembers)
const {
  currentPage: transactionPage,
  pageSize: transactionPageSize,
  paginatedRecords: paginatedTransactions
} = useTablePagination(filteredTransactions)
const {
  currentPage: refundPage,
  pageSize: refundPageSize,
  paginatedRecords: paginatedRefunds
} = useTablePagination(filteredRefunds)
const detailTransactions = computed(() =>
  salonStore.transactions.filter(item => item.memberId === selectedMember.value?.id)
)
const detailRefunds = computed(() =>
  salonStore.refundRecords.filter(item => item.memberId === selectedMember.value?.id)
)
function transactionMemberPhone(memberId: string | null) {
  return memberId ? (memberPhoneById.value.get(memberId) ?? '') : ''
}
const openMember = () => {
  Object.assign(memberForm, {
    name: '',
    phone: '',
    initialBalance: 0,
    giftAmount: 0,
    employee: ''
  })
  modal.value = 'member'
}
const openTransaction = (memberId = '') => {
  Object.assign(transactionForm, {
    memberId,
    type: 'recharge',
    amount: 0,
    giftAmount: 0,
    paymentMethod: '微信支付',
    item: '会员充值',
    employee: '',
    note: ''
  })
  modal.value = 'transaction'
}
const openDetail = (member: Member) => {
  selectedMember.value = member
  modal.value = 'detail'
}
const openAccountRefund = (member: Member) => {
  selectedMember.value = member
  const latestRecharge = [...salonStore.transactions]
    .filter(
      item => item.memberId === member.id && item.type === 'recharge' && item.status !== 'cancelled'
    )
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]
  Object.assign(accountRefundForm, {
    amount: member.refundablePrincipal,
    employee: latestRecharge?.employee ?? '',
    note: ''
  })
  modal.value = 'accountRefund'
}

watch(
  () => route.query.action,
  action => {
    if (action === 'new') openMember()
    if (action === 'recharge') openTransaction()
  },
  { immediate: true }
)

watch(activeTab, value => router.replace({ query: value === 'members' ? {} : { tab: value } }))

async function submitMember() {
  if (!(await validateForm(memberFormRef.value))) return
  saving.value = true
  try {
    await addMember({ ...memberForm })
    modal.value = null
    notify('会员档案创建成功')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function submitTransaction() {
  if (!(await validateForm(transactionFormRef.value))) return
  saving.value = true
  try {
    await addTransaction({ ...transactionForm })
    modal.value = null
    notify('充值已成功入账')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function submitAccountRefund() {
  const member = selectedMember.value
  if (!member) return
  if (!(await validateForm(accountRefundFormRef.value))) return
  const refundAmount = accountRefundForm.amount
  const isFullRefund = refundAmount >= member.refundablePrincipal - 0.001
  const consumedGiftOffset = isFullRefund ? Math.max(0, member.principalBalance - refundAmount) : 0
  const principalAfter = isFullRefund ? 0 : Math.max(0, member.principalBalance - refundAmount)
  const giftAfter = isFullRefund ? 0 : member.giftBalance
  const message = [
    `确认退还本金 ${currency(refundAmount)} 吗？`,
    `本次退款将冲减 ${accountRefundForm.employee} 在退款月份的充值绩效。`,
    consumedGiftOffset > 0 ? `已消费赠送金额将从本金中扣除 ${currency(consumedGiftOffset)}。` : '',
    isFullRefund && member.giftBalance > 0
      ? `同时清除赠送余额 ${currency(member.giftBalance)}。`
      : '',
    `退款后账户余额为 ${currency(principalAfter + giftAfter)}，会员资料和历史记录仍会保留，此操作不可撤销。`
  ]
    .filter(Boolean)
    .join('\n')
  try {
    await ElMessageBox.confirm(message, '二次确认账户退款', {
      type: 'warning',
      confirmButtonText: `确认退款 ${currency(refundAmount)}`,
      cancelButtonText: '取消'
    })
  } catch {
    return
  }
  saving.value = true
  try {
    await refundMemberAccount({
      memberId: member.id,
      amount: refundAmount,
      employee: accountRefundForm.employee,
      note: accountRefundForm.note
    })
    modal.value = null
    notify(`账户退款已登记，本次退款 ${currency(refundAmount)}`)
  } catch (reason) {
    notify(errorMessage(reason, '账户退款失败'), 'error')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <el-radio-group v-model="activeTab" class="salon-radio-group">
        <el-radio-button value="members">会员档案</el-radio-button>
        <el-radio-button value="transactions">资金流水</el-radio-button>
        <el-radio-button value="refunds">退款记录</el-radio-button>
      </el-radio-group>
      <div class="toolbar-actions">
        <el-button @click="openTransaction()">
          <WalletCards :size="17" />
          会员充值
        </el-button>
        <el-button type="primary" @click="openMember">
          <Plus :size="17" />
          新增会员
        </el-button>
      </div>
    </section>

    <section class="summary-strip">
      <div>
        <span class="summary-icon rose"><UserPlus :size="20" /></span>
        <p>
          会员总数
          <strong>{{ salonStore.members.length }}</strong>
          <small>当前有效档案</small>
        </p>
      </div>
      <div>
        <span class="summary-icon gold"><WalletCards :size="20" /></span>
        <p>
          储值余额
          <strong>{{ currency(salonStore.members.reduce((s, m) => s + m.balance, 0)) }}</strong>
          <small>所有会员合计</small>
        </p>
      </div>
      <div>
        <span class="summary-icon green"><CircleDollarSign :size="20" /></span>
        <p>
          累计消费
          <strong>
            {{ currency(salonStore.members.reduce((s, m) => s + m.totalConsumption, 0)) }}
          </strong>
          <small>历史消费总额</small>
        </p>
      </div>
    </section>

    <section class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <template v-if="activeTab === 'members'">
          <label class="table-filter-field">
            <span>会员姓名</span>
            <el-input
              v-model="memberFilters.name"
              clearable
              class="table-filter-input"
              placeholder="请输入会员姓名"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
          <label class="table-filter-field">
            <span>手机号</span>
            <el-input
              v-model="memberFilters.phone"
              clearable
              class="table-filter-input"
              placeholder="请输入手机号"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
        </template>
        <template v-else>
          <label class="table-filter-field">
            <span>会员姓名</span>
            <el-input
              v-model="transactionFilters.memberName"
              clearable
              class="table-filter-input"
              placeholder="请输入会员姓名"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
          <label class="table-filter-field">
            <span>项目</span>
            <el-input
              v-model="transactionFilters.item"
              clearable
              class="table-filter-input"
              placeholder="请输入项目"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
          <label class="table-filter-field">
            <span>备注</span>
            <el-input
              v-model="transactionFilters.note"
              clearable
              class="table-filter-input"
              placeholder="请输入备注"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
          <label class="table-filter-field">
            <span>经办员工</span>
            <el-input
              v-model="transactionFilters.employee"
              clearable
              class="table-filter-input"
              placeholder="请输入员工姓名"
            >
              <template #prefix><Search :size="16" /></template>
            </el-input>
          </label>
        </template>
      </div>
      <el-table
        v-if="activeTab === 'members'"
        :data="paginatedMembers"
        row-key="id"
        stripe
        class="salon-table"
      >
        <el-table-column label="会员信息" min-width="130">
          <template #default="{ row: member }">
            <div class="member-cell">
              <span>{{ member.name.slice(0, 1) }}</span>
              <div>
                <b>{{ member.name }}</b>
                <small>{{ member.phone }}</small>
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="账户余额" min-width="120">
          <template #default="{ row: member }">
            <strong class="money">{{ currency(member.balance) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="实付本金余额" min-width="125">
          <template #default="{ row: member }">{{ currency(member.principalBalance) }}</template>
        </el-table-column>
        <el-table-column label="赠送余额" min-width="110">
          <template #default="{ row: member }">{{ currency(member.giftBalance) }}</template>
        </el-table-column>
        <el-table-column label="累计充值" min-width="115">
          <template #default="{ row: member }">{{ currency(member.totalRecharge) }}</template>
        </el-table-column>
        <el-table-column label="累计消费" min-width="115">
          <template #default="{ row: member }">{{ currency(member.totalConsumption) }}</template>
        </el-table-column>
        <el-table-column label="最近到店" min-width="165">
          <template #default="{ row: member }">{{ fullDateTime(member.lastVisit) }}</template>
        </el-table-column>
        <el-table-column label="操作" min-width="176" fixed="right">
          <template #default="{ row: member }">
            <div class="element-row-actions">
              <el-tooltip content="查看详情">
                <el-button circle size="small" @click="openDetail(member as Member)">
                  <Eye :size="15" />
                </el-button>
              </el-tooltip>
              <el-tooltip content="会员充值">
                <el-button circle size="small" @click="openTransaction(member.id)">
                  <WalletCards :size="15" />
                </el-button>
              </el-tooltip>
              <el-tooltip v-if="authStore.user?.role === 'manager'" content="账户退款">
                <el-button
                  circle
                  size="small"
                  type="danger"
                  plain
                  :disabled="member.refundablePrincipal <= 0"
                  @click="openAccountRefund(member as Member)"
                >
                  <RotateCcw :size="15" />
                </el-button>
              </el-tooltip>
            </div>
          </template>
        </el-table-column>
      </el-table>
      <el-table
        v-else-if="activeTab === 'transactions'"
        :data="paginatedTransactions"
        row-key="id"
        stripe
        class="salon-table"
      >
        <el-table-column label="流水时间" min-width="130">
          <template #default="{ row }">{{ dateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="会员" min-width="145">
          <template #default="{ row }">
            <div>{{ row.memberName }}</div>
            <div v-if="transactionMemberPhone(row.memberId)" class="service-customer-phone">
              {{ transactionMemberPhone(row.memberId) }}
            </div>
          </template>
        </el-table-column>
        <el-table-column label="类型" min-width="80">
          <template #default="{ row }">
            <el-tag round :type="row.type === 'recharge' ? 'success' : 'danger'">
              {{ row.type === 'recharge' ? '充值' : '消费' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="item" label="项目" min-width="150" show-overflow-tooltip />
        <el-table-column prop="note" label="备注" min-width="140" show-overflow-tooltip />
        <el-table-column prop="employee" label="服务员工" min-width="100" />
        <el-table-column prop="paymentMethod" label="支付方式" min-width="105" />
        <el-table-column label="状态" min-width="90">
          <template #default="{ row }">
            <el-tag round :type="row.status === 'cancelled' ? 'info' : 'success'">
              {{ row.status === 'cancelled' ? '已撤销' : '有效' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="金额" min-width="115">
          <template #default="{ row }">
            <strong :class="row.type">
              {{ row.type === 'recharge' ? '+' : '-' }}{{ currency(row.amount) }}
            </strong>
          </template>
        </el-table-column>
        <el-table-column label="赠送金额" min-width="105">
          <template #default="{ row }">
            {{ row.type === 'recharge' && row.giftAmount ? currency(row.giftAmount) : '--' }}
          </template>
        </el-table-column>
        <el-table-column label="交易后余额" min-width="120">
          <template #default="{ row }">{{ currency(row.balanceAfter) }}</template>
        </el-table-column>
      </el-table>
      <el-table v-else :data="paginatedRefunds" row-key="id" stripe class="salon-table">
        <el-table-column label="退款时间" min-width="145">
          <template #default="{ row }">{{ dateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="会员" min-width="140">
          <template #default="{ row }">
            <div>{{ row.memberName }}</div>
            <div v-if="transactionMemberPhone(row.memberId)" class="service-customer-phone">
              {{ transactionMemberPhone(row.memberId) }}
            </div>
          </template>
        </el-table-column>
        <el-table-column label="类型" min-width="100">
          <template #default="{ row }">
            <el-tag round type="danger">
              {{ row.refundType === 'account' ? '账户退款' : '套餐退款' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="退款金额" min-width="115">
          <template #default="{ row }">
            <strong class="consume">-{{ currency(row.amount) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="退回本金余额" min-width="125">
          <template #default="{ row }">{{ currency(row.balanceAmount) }}</template>
        </el-table-column>
        <el-table-column label="线下退款" min-width="110">
          <template #default="{ row }">{{ currency(row.cashAmount) }}</template>
        </el-table-column>
        <el-table-column label="赠送金扣除" min-width="115">
          <template #default="{ row }">
            {{ row.giftForfeitedAmount ? currency(row.giftForfeitedAmount) : '--' }}
          </template>
        </el-table-column>
        <el-table-column prop="operator" label="操作人" min-width="105" />
        <el-table-column prop="note" label="备注" min-width="180" show-overflow-tooltip />
        <el-table-column label="退款后余额" min-width="120">
          <template #default="{ row }">{{ currency(row.balanceAfter) }}</template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-if="activeTab === 'members'"
        v-model="memberPage"
        v-model:page-size="memberPageSize"
        :total="filteredMembers.length"
      />
      <TablePagination
        v-else-if="activeTab === 'transactions'"
        v-model="transactionPage"
        v-model:page-size="transactionPageSize"
        :total="filteredTransactions.length"
      />
      <TablePagination
        v-else
        v-model="refundPage"
        v-model:page-size="refundPageSize"
        :total="filteredRefunds.length"
      />
    </section>

    <BaseModal
      v-if="modal === 'member'"
      title="新增会员"
      subtitle="建立会员档案，可选择同时开卡充值"
      @close="modal = null"
    >
      <el-form
        ref="memberFormRef"
        :model="memberForm"
        :rules="memberRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item label="会员姓名" prop="name">
            <el-input v-model="memberForm.name" clearable placeholder="请输入姓名" />
          </el-form-item>
          <el-form-item label="手机号码" prop="phone">
            <el-input
              v-model="memberForm.phone"
              clearable
              maxlength="20"
              placeholder="请输入手机号"
            />
          </el-form-item>
          <el-form-item label="开卡员工" prop="employee">
            <EmployeeSelect v-model="memberForm.employee" />
          </el-form-item>
          <el-form-item label="开卡充值金额" prop="initialBalance">
            <el-input-number
              v-model="memberForm.initialBalance"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="赠送金额" prop="giftAmount">
            <el-input-number
              v-model="memberForm.giftAmount"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
        </div>
      </el-form>
      <div v-if="memberForm.initialBalance || memberForm.giftAmount" class="form-tip">
        本次账户新增金额：{{
          currency(memberForm.initialBalance + memberForm.giftAmount)
        }}，提成仅按实际充值金额计算。
      </div>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitMember">保存会员</el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="modal === 'transaction'"
      title="会员充值"
      subtitle="每一笔资金变化都会生成可追溯流水"
      @close="modal = null"
    >
      <el-form
        ref="transactionFormRef"
        :model="transactionForm"
        :rules="transactionRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item class="span-2" label="选择会员" prop="memberId">
            <MemberSelect v-model="transactionForm.memberId" />
          </el-form-item>
          <el-form-item label="充值金额" prop="amount">
            <el-input-number
              v-model="transactionForm.amount"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="赠送金额" prop="giftAmount">
            <el-input-number
              v-model="transactionForm.giftAmount"
              :min="0"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item label="支付方式" prop="paymentMethod">
            <el-select v-model="transactionForm.paymentMethod">
              <el-option
                v-for="item in RECHARGE_PAYMENT_METHODS"
                :key="item"
                :label="item"
                :value="item"
              />
            </el-select>
          </el-form-item>
          <el-form-item label="充值归属员工" prop="employee">
            <EmployeeSelect v-model="transactionForm.employee" />
          </el-form-item>
          <el-form-item class="span-2" label="备注">
            <el-input
              v-model="transactionForm.note"
              type="textarea"
              :rows="3"
              maxlength="200"
              show-word-limit
              placeholder="选填"
            />
          </el-form-item>
        </div>
      </el-form>
      <div class="form-tip">
        本次账户新增金额：{{
          currency(transactionForm.amount + transactionForm.giftAmount)
        }}，充值本金计入所选员工绩效，赠送金额不计营收和提成。
      </div>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitTransaction">确认入账</el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="modal === 'detail' && selectedMember"
      title="会员详情"
      :subtitle="`${selectedMember.name} · ${selectedMember.phone}`"
      wide
      @close="modal = null"
    >
      <div class="detail-metrics">
        <div>
          <span>账户余额</span>
          <strong>{{ currency(selectedMember.balance) }}</strong>
        </div>
        <div>
          <span>累计充值</span>
          <strong>{{ currency(selectedMember.totalRecharge) }}</strong>
        </div>
        <div>
          <span>实付本金余额</span>
          <strong>{{ currency(selectedMember.principalBalance) }}</strong>
        </div>
        <div>
          <span>赠送余额</span>
          <strong>{{ currency(selectedMember.giftBalance) }}</strong>
        </div>
        <div>
          <span>当前可退本金</span>
          <strong>{{ currency(selectedMember.refundablePrincipal) }}</strong>
        </div>
        <div>
          <span>累计消费</span>
          <strong>{{ currency(selectedMember.totalConsumption) }}</strong>
        </div>
        <div>
          <span>入会日期</span>
          <strong>{{ dateOnly(selectedMember.joinDate) }}</strong>
        </div>
      </div>
      <h4 class="subheading">最近资金流水</h4>
      <div class="simple-list">
        <div v-for="item in detailTransactions" :key="item.id">
          <span class="type-badge" :class="item.type">
            {{ item.type === 'recharge' ? '充值' : '消费' }}
          </span>
          <p>
            <b>{{ item.item }}</b>
            <small>
              {{ dateTime(item.createdAt) }} · {{ item.employee }}
              <template v-if="item.status === 'cancelled'">· 已撤销</template>
            </small>
          </p>
          <strong :class="item.type">
            {{ item.type === 'recharge' ? '+' : '-' }}{{ currency(item.amount) }}
          </strong>
        </div>
      </div>
      <template v-if="detailRefunds.length">
        <h4 class="subheading">退款记录</h4>
        <div class="simple-list">
          <div v-for="item in detailRefunds" :key="item.id">
            <span class="type-badge consume">退款</span>
            <p>
              <b>{{ item.refundType === 'account' ? '账户退款' : '套餐退款' }}</b>
              <small>{{ dateTime(item.createdAt) }} · {{ item.operator }} · {{ item.note }}</small>
            </p>
            <strong class="consume">-{{ currency(item.amount) }}</strong>
          </div>
        </div>
      </template>
    </BaseModal>

    <BaseModal
      v-if="modal === 'accountRefund' && selectedMember"
      title="会员账户退款"
      :subtitle="`${selectedMember.name} · ${selectedMember.phone}`"
      @close="modal = null"
    >
      <div class="detail-metrics">
        <div>
          <span>当前本金余额</span>
          <strong>{{ currency(selectedMember.principalBalance) }}</strong>
        </div>
        <div>
          <span>已消费赠送金抵扣</span>
          <strong>
            {{
              currency(
                Math.max(0, selectedMember.principalBalance - selectedMember.refundablePrincipal)
              )
            }}
          </strong>
        </div>
        <div>
          <span>当前可退本金</span>
          <strong class="money">{{ currency(selectedMember.refundablePrincipal) }}</strong>
        </div>
        <div>
          <span>{{ accountRefundIsFull ? '将清除赠送余额' : '将保留赠送余额' }}</span>
          <strong>{{ currency(selectedMember.giftBalance) }}</strong>
        </div>
      </div>
      <el-form
        ref="accountRefundFormRef"
        :model="accountRefundForm"
        :rules="accountRefundRules"
        label-position="top"
        style="margin-top: 10px"
        scroll-to-error
      >
        <div class="form-grid">
          <el-form-item label="本次退款金额" prop="amount">
            <el-input-number
              v-model="accountRefundForm.amount"
              :min="0.01"
              :max="selectedMember.refundablePrincipal"
              :precision="2"
              :controls="false"
              align="left"
            />
            <div class="form-tip">最多可退 {{ currency(selectedMember.refundablePrincipal) }}</div>
          </el-form-item>
          <el-form-item label="绩效归属员工" prop="employee">
            <EmployeeSelect
              v-model="accountRefundForm.employee"
              :active-only="false"
              placeholder="请选择需要冲减绩效的员工"
            />
          </el-form-item>
        </div>
        <el-form-item label="退款备注">
          <el-input
            v-model="accountRefundForm.note"
            type="textarea"
            :rows="3"
            maxlength="200"
            show-word-limit
            placeholder="选填"
          />
        </el-form-item>
      </el-form>
      <div class="form-tip">
        部分退款仅扣减本次退款本金；退还全部可退本金时，剩余赠送余额将同时清零。
      </div>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="danger" :loading="saving" @click="submitAccountRefund">确认退款</el-button>
      </template>
    </BaseModal>
  </div>
</template>
