<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus, ReceiptText, Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import { addSupplyPurchase, deleteSupplyPurchase, salonStore } from '../data/repository'
import type { SupplyPurchase, SupplyPurchaseInput } from '../types'
import { currency } from '../utils'
import { errorMessage, notify } from '../utils/feedback'
import { nonNegativeNumberRule, requiredTextRule, validateForm } from '../utils/validation'

const today = new Date().toISOString().slice(0, 10)
const filterForm = reactive({ dateRange: [] as string[] })
const appliedDateRange = ref<string[]>([])
const modalVisible = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()
const form = reactive<SupplyPurchaseInput>({
  name: '',
  purchasedAt: today,
  amount: 0
})
const rules: FormRules<SupplyPurchaseInput> = {
  name: [requiredTextRule('请输入物料名称')],
  purchasedAt: [requiredTextRule('请选择采购日期')],
  amount: [nonNegativeNumberRule('采购价格不能小于0')]
}

const purchaseRecords = computed(() => {
  const [start, end] = appliedDateRange.value
  return salonStore.supplyPurchases.filter(
    item => (!start || item.purchasedAt >= start) && (!end || item.purchasedAt <= end)
  )
})
const totalAmount = computed(() =>
  purchaseRecords.value.reduce((sum, item) => sum + item.amount, 0)
)

function query() {
  appliedDateRange.value = [...filterForm.dateRange]
}

function reset() {
  filterForm.dateRange = []
  query()
}

function openCreate() {
  Object.assign(form, { name: '', purchasedAt: today, amount: 0 })
  modalVisible.value = true
}

async function submit() {
  if (!(await validateForm(formRef.value))) return
  saving.value = true
  try {
    await addSupplyPurchase({ ...form })
    modalVisible.value = false
    notify('采购记录已保存')
  } catch (reason) {
    notify(errorMessage(reason, '保存失败'), 'error')
  } finally {
    saving.value = false
  }
}

async function removeRecord(item: SupplyPurchase) {
  try {
    await ElMessageBox.confirm(`确认删除“${item.name}”的采购记录吗？`, '删除采购记录', {
      type: 'warning',
      confirmButtonText: '确认删除',
      cancelButtonText: '取消'
    })
    await deleteSupplyPurchase(item.id)
    notify('采购记录已删除')
  } catch (reason) {
    if (reason !== 'cancel' && reason !== 'close') notify(errorMessage(reason, '删除失败'), 'error')
  }
}
</script>

<template>
  <div class="page material-page">
    <section class="purchase-summary">
      <div class="summary-icon"><ReceiptText :size="22" /></div>
      <div>
        <span>物料采购总金额</span>
        <strong>{{ currency(totalAmount) }}</strong>
      </div>
      <small>共 {{ purchaseRecords.length }} 条采购记录</small>
    </section>

    <section class="panel table-panel material-panel">
      <div class="material-toolbar">
        <div>
          <h3>采购记录</h3>
          <p>登记门店日常物料采购及实际采购价格</p>
        </div>
        <el-button type="primary" @click="openCreate">
          <Plus :size="17" />
          登记采购
        </el-button>
      </div>

      <div class="purchase-filter">
        <label class="filter-field">
          <span>采购日期</span>
          <el-date-picker
            v-model="filterForm.dateRange"
            type="daterange"
            value-format="YYYY-MM-DD"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
          />
        </label>
        <div class="filter-actions">
          <el-button type="primary" @click="query">
            <Search :size="15" />
            查询
          </el-button>
          <el-button @click="reset">重置</el-button>
        </div>
      </div>

      <el-table
        :data="purchaseRecords"
        row-key="id"
        stripe
        class="salon-table material-table"
        empty-text="当前条件下暂无采购记录"
      >
        <el-table-column prop="name" label="物料名称" min-width="240" />
        <el-table-column prop="purchasedAt" label="采购日期" min-width="160" />
        <el-table-column label="采购价格" min-width="160">
          <template #default="{ row }">
            <strong class="purchase-price">{{ currency(row.amount) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              type="danger"
              plain
              @click="removeRecord(row as SupplyPurchase)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <BaseModal
      v-if="modalVisible"
      title="物料采购登记"
      subtitle="记录物料名称、采购日期和实际采购价格"
      @close="modalVisible = false"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top" scroll-to-error>
        <el-form-item label="物料名称" prop="name">
          <el-input v-model="form.name" clearable placeholder="例如：矿泉水、抽纸、零食" />
        </el-form-item>
        <div class="form-grid">
          <el-form-item label="采购日期" prop="purchasedAt">
            <el-date-picker
              v-model="form.purchasedAt"
              type="date"
              value-format="YYYY-MM-DD"
              placeholder="请选择采购日期"
            />
          </el-form-item>
          <el-form-item label="采购价格" prop="amount">
            <el-input-number
              v-model="form.amount"
              :min="0"
              :precision="2"
              :controls="false"
              align="left"
            />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="modalVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submit">保存记录</el-button>
      </template>
    </BaseModal>
  </div>
</template>

<style scoped>
.material-page {
  display: block;
}

.material-page > * + * {
  margin-top: 16px;
}

.purchase-summary {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 20px 24px;
  background: linear-gradient(135deg, #fff 0%, #fff7f4 100%);
  border: 1px solid #eee5e1;
  border-radius: 16px;
  box-shadow: 0 8px 24px rgb(77 45 35 / 5%);
}

.summary-icon {
  display: grid;
  width: 46px;
  height: 46px;
  place-items: center;
  color: var(--rose);
  background: #fce9e3;
  border-radius: 13px;
}

.purchase-summary div:nth-child(2) {
  display: grid;
  gap: 3px;
}

.purchase-summary span,
.purchase-summary small {
  color: var(--muted);
}

.purchase-summary strong {
  color: #30282a;
  font-size: 26px;
  line-height: 1.2;
}

.purchase-summary small {
  margin-left: auto;
}

.material-panel {
  overflow: hidden;
}

.material-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 18px 0;
  margin-bottom: 18px;
}

.material-toolbar h3,
.material-toolbar p {
  margin: 0;
}

.purchase-filter {
  display: flex;
  align-items: end;
  gap: 16px;
  margin: 0 16px 20px;
  padding: 16px;
  background: #faf8f7;
  border: 1px solid #eee8e5;
  border-radius: 12px;
}

.filter-field {
  display: grid;
  flex: 1;
  gap: 7px;
  max-width: 520px;
}

.filter-field > span {
  color: var(--muted);
  font-size: 13px;
  font-weight: 600;
}

.filter-field :deep(.el-date-editor) {
  width: 100%;
}

.filter-actions {
  display: flex;
  gap: 8px;
}

.material-toolbar p {
  margin-top: 4px;
  color: var(--muted);
  font-size: 13px;
}

.purchase-price {
  color: var(--rose);
}

@media (max-width: 720px) {
  .purchase-summary {
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .purchase-summary small {
    width: 100%;
    margin-left: 60px;
  }

  .material-toolbar {
    align-items: stretch;
    flex-direction: column;
  }

  .purchase-filter {
    align-items: stretch;
    flex-direction: column;
  }

  .filter-actions .el-button {
    flex: 1;
  }
}
</style>
