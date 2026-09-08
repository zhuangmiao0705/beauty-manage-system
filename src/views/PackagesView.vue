<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessageBox } from 'element-plus'
import 'element-plus/es/components/message-box/style/css'
import type { FormInstance, FormRules } from 'element-plus'
import { Eye, MinusCircle, PackagePlus, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import EmployeeSelect from '../components/EmployeeSelect.vue'
import MemberSelect from '../components/MemberSelect.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { PACKAGE_PAYMENT_METHODS, PACKAGE_TYPES } from '../config/options'
import { consumePackage, purchasePackage, salonStore } from '../data/repository'
import type {
  PackageConsumptionInput,
  PackagePurchase,
  PackagePurchaseInput,
  PackageType
} from '../types'
import {
  currency,
  fullDateTime,
  packageDefinitionLimitText,
  packagePurchaseLimitText
} from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { positiveNumberRule, validateForm } from '../utils/validation'

type PurchaseFilters = {
  memberPhone: string
  employee: string
  packageId: string
  packageType: PackageType | ''
  status: string
}
const purchaseFilterForm = reactive<PurchaseFilters>({
  memberPhone: '',
  employee: '',
  packageId: '',
  packageType: '',
  status: ''
})
const appliedPurchaseFilters = reactive<PurchaseFilters>({ ...purchaseFilterForm })
const modal = ref<'purchase' | 'consume' | 'flows' | null>(null)
const saving = ref(false)
const selectedPurchase = ref<PackagePurchase | null>(null)
const purchaseFormRef = ref<FormInstance>()
const consumeFormRef = ref<FormInstance>()
const purchaseForm = reactive<PackagePurchaseInput>({
  memberId: '',
  packageId: '',
  employee: '',
  paymentMethod: '会员余额',
  balancePaymentAmount: 0,
  cashPaymentAmount: 0
})
const consumeForm = reactive<PackageConsumptionInput>({
  purchaseId: '',
  employee: '',
  duration: 60,
  note: ''
})
const purchaseRules = computed<FormRules<PackagePurchaseInput>>(() => ({
  memberId: [{ required: true, message: '请选择会员', trigger: 'change' }],
  packageId: [{ required: true, message: '请选择套餐种类', trigger: 'change' }],
  employee: [{ required: true, message: '请选择销售员工', trigger: 'change' }],
  paymentMethod: [{ required: true, message: '请选择支付方式', trigger: 'change' }],
  balancePaymentAmount: [
    {
      required: purchaseForm.paymentMethod !== '现金',
      trigger: ['blur', 'change'],
      validator: (_rule, value, callback) => {
        if (!selectedPackage.value || purchaseForm.paymentMethod === '现金') return callback()
        if (selectedPackage.value.price === 0) return callback()
        const amount = Number(value)
        if (!Number.isFinite(amount) || amount <= 0)
          return callback(new Error('余额支付金额必须大于0'))
        if (
          purchaseForm.paymentMethod === '余额现金组合支付' &&
          amount >= selectedPackage.value.price
        )
          return callback(new Error('组合支付时余额金额必须小于套餐价格'))
        if (selectedMember.value && amount > selectedMember.value.principalBalance)
          return callback(new Error('实付本金余额不足，赠送余额不可购买套餐'))
        callback()
      }
    }
  ],
  cashPaymentAmount: [
    {
      required: purchaseForm.paymentMethod !== '会员余额',
      trigger: 'change',
      validator: (_rule, value, callback) => {
        if (!selectedPackage.value || purchaseForm.paymentMethod === '会员余额') return callback()
        if (selectedPackage.value.price === 0) return callback()
        const cashAmount = Number(value)
        if (!Number.isFinite(cashAmount) || cashAmount <= 0)
          return callback(new Error('现金支付金额必须大于0'))
        const total = Math.round((cashAmount + purchaseForm.balancePaymentAmount) * 100) / 100
        if (total < selectedPackage.value.price)
          return callback(
            new Error(
              `支付总金额不足套餐价格，还差${currency(selectedPackage.value.price - total)}`
            )
          )
        if (total > selectedPackage.value.price)
          return callback(new Error('支付总金额不能超过套餐价格'))
        callback()
      }
    }
  ]
}))
const consumeRules: FormRules<PackageConsumptionInput> = {
  employee: [{ required: true, message: '请选择本次服务员工', trigger: 'change' }],
  duration: [positiveNumberRule('请输入大于 0 的服务时长')]
}
const activePackages = computed(() => salonStore.packages.filter(item => item.status === 'active'))
const selectedPackage = computed(() =>
  salonStore.packages.find(item => item.id === purchaseForm.packageId)
)
const selectedMember = computed(() =>
  salonStore.members.find(item => item.id === purchaseForm.memberId)
)

watch(
  () => [purchaseForm.paymentMethod, purchaseForm.packageId, purchaseForm.balancePaymentAmount],
  () => {
    const price = selectedPackage.value?.price ?? 0
    if (purchaseForm.paymentMethod === '会员余额') {
      purchaseForm.balancePaymentAmount = price
      purchaseForm.cashPaymentAmount = 0
    } else if (purchaseForm.paymentMethod === '现金') {
      purchaseForm.balancePaymentAmount = 0
      purchaseForm.cashPaymentAmount = price
    } else {
      purchaseForm.cashPaymentAmount = Math.max(
        0,
        Math.round((price - Number(purchaseForm.balancePaymentAmount || 0)) * 100) / 100
      )
    }
  }
)
const filteredPurchases = computed(() =>
  salonStore.packagePurchases.filter(item => {
    const phone = appliedPurchaseFilters.memberPhone.trim()
    if (phone && !memberPhone(item.memberId).includes(phone)) return false
    if (appliedPurchaseFilters.employee && item.employee !== appliedPurchaseFilters.employee)
      return false
    if (appliedPurchaseFilters.packageId && item.packageId !== appliedPurchaseFilters.packageId)
      return false
    if (
      appliedPurchaseFilters.packageType &&
      item.packageType !== appliedPurchaseFilters.packageType
    )
      return false
    if (appliedPurchaseFilters.status && item.status !== appliedPurchaseFilters.status) return false
    return true
  })
)
const consumptionFlows = computed(() =>
  salonStore.packageConsumptions.filter(item => item.purchaseId === selectedPurchase.value?.id)
)
const {
  currentPage: purchasePage,
  pageSize: purchasePageSize,
  paginatedRecords: paginatedPurchases
} = useTablePagination(filteredPurchases)
const {
  currentPage: flowPage,
  pageSize: flowPageSize,
  paginatedRecords: paginatedConsumptionFlows
} = useTablePagination(consumptionFlows)

function memberPhone(memberId: string) {
  return salonStore.members.find(item => item.id === memberId)?.phone ?? ''
}

function queryPurchases() {
  Object.assign(appliedPurchaseFilters, purchaseFilterForm)
}

function resetPurchaseFilters() {
  Object.assign(purchaseFilterForm, {
    memberPhone: '',
    employee: '',
    packageId: '',
    packageType: '',
    status: ''
  })
  queryPurchases()
}

function openPurchase() {
  Object.assign(purchaseForm, {
    memberId: '',
    packageId: '',
    employee: '',
    paymentMethod: '会员余额',
    balancePaymentAmount: 0,
    cashPaymentAmount: 0
  })
  modal.value = 'purchase'
}

function openConsume(purchase: PackagePurchase) {
  selectedPurchase.value = purchase
  Object.assign(consumeForm, { purchaseId: purchase.id, employee: '', duration: 60, note: '' })
  modal.value = 'consume'
}

function openFlows(purchase: PackagePurchase) {
  selectedPurchase.value = purchase
  modal.value = 'flows'
}

async function submitPurchase() {
  if (!(await validateForm(purchaseFormRef.value))) return
  saving.value = true
  try {
    await purchasePackage({ ...purchaseForm })
    modal.value = null
    notify('套餐购买记录已保存')
  } catch (reason) {
    notify(errorMessage(reason, '购买失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function submitConsumption() {
  if (!(await validateForm(consumeFormRef.value)) || !selectedPurchase.value) return
  try {
    await ElMessageBox.confirm(
      selectedPurchase.value.limitType === 'time'
        ? `确认登记“${selectedPurchase.value.packageName}”本次消费吗？该套餐在有效期内不限次数。`
        : `确认消耗“${selectedPurchase.value.packageName}”1次吗？确认后剩余次数将减1。`,
      '确认套餐消耗',
      { type: 'warning', confirmButtonText: '确认消耗', cancelButtonText: '取消' }
    )
  } catch {
    return
  }
  saving.value = true
  try {
    await consumePackage({ ...consumeForm })
    modal.value = null
    notify('套餐已成功消耗1次')
  } catch (reason) {
    notify(errorMessage(reason, '消耗失败'), 'error')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <div />
      <el-button type="primary" @click="openPurchase">
        <PackagePlus :size="17" />
        登记套餐购买
      </el-button>
    </section>

    <section class="panel table-panel">
      <div class="table-toolbar element-toolbar table-filter-toolbar">
        <label class="table-filter-field">
          <span>会员手机号</span>
          <el-input
            v-model="purchaseFilterForm.memberPhone"
            clearable
            class="table-filter-input"
            placeholder="请输入会员手机号"
          />
        </label>
        <label class="table-filter-field">
          <span>销售员工</span>
          <EmployeeSelect
            v-model="purchaseFilterForm.employee"
            :active-only="false"
            class="table-filter-select"
          />
        </label>
        <label class="table-filter-field">
          <span>套餐种类</span>
          <el-select
            v-model="purchaseFilterForm.packageId"
            clearable
            class="table-filter-select"
            placeholder="请选择套餐"
          >
            <el-option
              v-for="item in salonStore.packages"
              :key="item.id"
              :label="item.name"
              :value="item.id"
            />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>套餐类型</span>
          <el-select
            v-model="purchaseFilterForm.packageType"
            clearable
            class="table-filter-select"
            placeholder="请选择类型"
          >
            <el-option v-for="item in PACKAGE_TYPES" :key="item" :label="item" :value="item" />
          </el-select>
        </label>
        <label class="table-filter-field">
          <span>状态</span>
          <el-select
            v-model="purchaseFilterForm.status"
            clearable
            class="table-filter-select"
            placeholder="请选择状态"
          >
            <el-option label="使用中" value="active" />
            <el-option label="已结束" value="completed" />
          </el-select>
        </label>
        <div class="service-filter-actions">
          <el-button type="primary" @click="queryPurchases">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="resetPurchaseFilters">重置</el-button>
        </div>
      </div>
      <el-table :data="paginatedPurchases" row-key="id" stripe class="salon-table">
        <el-table-column label="会员信息" min-width="145">
          <template #default="{ row }">
            <div>{{ row.memberName }}</div>
            <div class="service-customer-phone">{{ memberPhone(row.memberId) }}</div>
          </template>
        </el-table-column>
        <el-table-column prop="employee" label="销售员工" min-width="105" />
        <el-table-column prop="packageName" label="套餐种类" min-width="145" />
        <el-table-column label="套餐类型" min-width="95">
          <template #default="{ row }">
            <el-tag round :type="row.packageType === '套盒' ? 'warning' : 'info'">
              {{ row.packageType }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="购买金额" min-width="110">
          <template #default="{ row }">{{ currency(row.price) }}</template>
        </el-table-column>
        <el-table-column prop="paymentMethod" label="支付方式" min-width="150" />
        <el-table-column label="现金支付" min-width="110">
          <template #default="{ row }">{{ currency(row.cashPaymentAmount) }}</template>
        </el-table-column>
        <el-table-column label="余额支付" min-width="110">
          <template #default="{ row }">{{ currency(row.balancePaymentAmount) }}</template>
        </el-table-column>
        <el-table-column label="使用限制" min-width="200">
          <template #default="{ row }">
            {{ packagePurchaseLimitText(row as PackagePurchase) }}
          </template>
        </el-table-column>
        <el-table-column label="最近消耗时间" min-width="165">
          <template #default="{ row }">
            {{ row.lastConsumedAt ? fullDateTime(row.lastConsumedAt) : '--' }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag round :type="row.status === 'active' ? 'success' : 'info'">
              {{ row.status === 'active' ? '使用中' : '已结束' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="190" fixed="right">
          <template #default="{ row }">
            <div class="element-row-actions">
              <el-button
                size="small"
                :disabled="row.status !== 'active'"
                @click="openConsume(row as PackagePurchase)"
              >
                <MinusCircle :size="14" />
                消耗
              </el-button>
              <el-button size="small" @click="openFlows(row as PackagePurchase)">
                <Eye :size="14" />
                查看流水
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="purchasePage"
        v-model:page-size="purchasePageSize"
        :total="filteredPurchases.length"
      />
    </section>

    <BaseModal
      v-if="modal === 'purchase'"
      title="登记套餐购买"
      subtitle="套餐价格以当前配置为准，不支持临时优惠"
      @close="modal = null"
    >
      <el-form
        ref="purchaseFormRef"
        :model="purchaseForm"
        :rules="purchaseRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item class="span-2" label="选择会员" prop="memberId">
            <MemberSelect v-model="purchaseForm.memberId" />
          </el-form-item>
          <el-form-item label="套餐种类" prop="packageId">
            <el-select v-model="purchaseForm.packageId" clearable placeholder="请选择套餐种类">
              <el-option
                v-for="item in activePackages"
                :key="item.id"
                :label="`${item.name} · ${item.packageType} · ${currency(item.price)} · ${packageDefinitionLimitText(item)}`"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
          <el-form-item label="销售员工" prop="employee">
            <EmployeeSelect v-model="purchaseForm.employee" />
          </el-form-item>
          <el-form-item label="支付方式" prop="paymentMethod">
            <el-select v-model="purchaseForm.paymentMethod">
              <el-option
                v-for="item in PACKAGE_PAYMENT_METHODS"
                :key="item"
                :label="item"
                :value="item"
              />
            </el-select>
          </el-form-item>
          <el-form-item label="固定价格">
            <el-input
              :model-value="selectedPackage ? currency(selectedPackage.price) : '--'"
              disabled
            />
          </el-form-item>
          <el-form-item label="余额支付金额" prop="balancePaymentAmount">
            <el-input-number
              v-model="purchaseForm.balancePaymentAmount"
              :disabled="purchaseForm.paymentMethod !== '余额现金组合支付'"
              :min="0"
              :precision="2"
              :controls="false"
              placeholder="请输入余额扣除金额"
              align="left"
            />
          </el-form-item>
          <el-form-item label="现金支付金额" prop="cashPaymentAmount">
            <el-input :model-value="currency(purchaseForm.cashPaymentAmount)" disabled />
          </el-form-item>
        </div>
      </el-form>
      <div v-if="selectedPackage && selectedMember" class="form-tip">
        本次购买 {{ selectedPackage.name }}（{{ selectedPackage.packageType }}），{{
          selectedPackage.limitType === 'time'
            ? `有效期 ${selectedPackage.validityDays} 天，期内不限次数`
            : `共 ${selectedPackage.totalUses} 次`
        }}；可用实付本金余额
        {{ currency(selectedMember.principalBalance) }}。赠送余额不可用于购买套餐。
      </div>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitPurchase">确认购买</el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="modal === 'consume' && selectedPurchase"
      title="套餐消耗"
      :subtitle="`${selectedPurchase.memberName} · ${selectedPurchase.packageName} · ${packagePurchaseLimitText(selectedPurchase)}`"
      @close="modal = null"
    >
      <el-form
        ref="consumeFormRef"
        :model="consumeForm"
        :rules="consumeRules"
        scroll-to-error
        label-position="top"
      >
        <div class="form-grid">
          <el-form-item class="span-2" label="本次服务员工" prop="employee">
            <EmployeeSelect v-model="consumeForm.employee" />
          </el-form-item>
          <el-form-item label="服务时长（分钟）" prop="duration">
            <el-input-number
              v-model="consumeForm.duration"
              :min="1"
              :controls="false"
              align="left"
            />
          </el-form-item>
          <el-form-item class="span-2" label="备注">
            <el-input
              v-model="consumeForm.note"
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
        确认后{{
          selectedPurchase.limitType === 'time' ? '登记本次消费' : '剩余次数减1'
        }}，并自动生成一条{{
          selectedPurchase.packageType === '套盒' ? '套盒手工' : '普通手工'
        }}服务及提成记录。
      </div>
      <template #footer>
        <el-button @click="modal = null">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submitConsumption">
          下一步确认
        </el-button>
      </template>
    </BaseModal>

    <BaseModal
      v-if="modal === 'flows' && selectedPurchase"
      title="套餐消耗流水"
      :subtitle="`${selectedPurchase.memberName} · ${selectedPurchase.packageName}`"
      wide
      @close="modal = null"
    >
      <el-table :data="paginatedConsumptionFlows" row-key="id" stripe>
        <el-table-column label="消耗时间" min-width="170">
          <template #default="{ row }">{{ fullDateTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column prop="employee" label="服务员工" min-width="120" />
        <el-table-column label="服务时长" min-width="100">
          <template #default="{ row }">{{ row.duration }} 分钟</template>
        </el-table-column>
        <el-table-column label="消耗后剩余" min-width="110">
          <template #default="{ row }">
            {{ selectedPurchase.limitType === 'time' ? '有效期内' : `${row.remainingAfter}次` }}
          </template>
        </el-table-column>
        <el-table-column prop="note" label="备注" min-width="160" show-overflow-tooltip />
      </el-table>
      <TablePagination
        v-model="flowPage"
        v-model:page-size="flowPageSize"
        :total="consumptionFlows.length"
      />
    </BaseModal>
  </div>
</template>
