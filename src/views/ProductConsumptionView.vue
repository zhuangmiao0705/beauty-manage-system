<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { Search } from 'lucide-vue-next'
import BaseModal from '../components/BaseModal.vue'
import { salonStore } from '../data/repository'
import { currency, fullDateTime } from '../utils'

type ConsumptionFilters = {
  productName: string
  dateRange: string[]
}

const filterForm = reactive<ConsumptionFilters>({ productName: '', dateRange: [] })
const appliedFilters = reactive<ConsumptionFilters>({ productName: '', dateRange: [] })
const detailVisible = ref(false)
const selectedProductId = ref('')
const selectedProductName = ref('')

const filteredConsumptions = computed(() => {
  const keyword = appliedFilters.productName.trim()
  const [start, end] = appliedFilters.dateRange
  return salonStore.productConsumptions.filter(item => {
    const day = item.consumedAt.slice(0, 10)
    return (
      item.status === 'active' &&
      (!keyword || item.productName.includes(keyword)) &&
      (!start || day >= start) &&
      (!end || day <= end)
    )
  })
})

const consumptionSummary = computed(() => {
  const totals = new Map<
    string,
    { productId: string; productName: string; quantity: number; cost: number }
  >()
  filteredConsumptions.value.forEach(item => {
    const current = totals.get(item.productId) ?? {
      productId: item.productId,
      productName: item.productName,
      quantity: 0,
      cost: 0
    }
    current.quantity += item.quantity
    current.cost += item.quantity * item.unitCost
    totals.set(item.productId, current)
  })
  return [...totals.values()].sort((a, b) => b.quantity - a.quantity)
})

const totalConsumptionCost = computed(() =>
  filteredConsumptions.value.reduce((sum, item) => sum + item.quantity * item.unitCost, 0)
)

const currentResultDetails = computed(() =>
  filteredConsumptions.value.filter(item => item.productId === selectedProductId.value)
)
const detailSubtitle = computed(() => {
  const [start, end] = appliedFilters.dateRange
  const range = start && end ? `${start} 至 ${end}` : '全部日期'
  return `${selectedProductName.value} · ${range} · 当前搜索结果`
})

function query() {
  appliedFilters.productName = filterForm.productName
  appliedFilters.dateRange = [...filterForm.dateRange]
}

function reset() {
  filterForm.productName = ''
  filterForm.dateRange = []
  query()
}

function openDetails(productId: string, productName: string) {
  selectedProductId.value = productId
  selectedProductName.value = productName
  detailVisible.value = true
}
</script>

<template>
  <div class="page material-page">
    <section class="panel table-panel material-panel">
      <div class="consumption-filter">
        <label class="filter-field">
          <span>产品名称</span>
          <el-input
            v-model="filterForm.productName"
            clearable
            placeholder="请输入产品名称"
            @keyup.enter="query"
          />
        </label>
        <label class="filter-field date-field">
          <span>消耗日期</span>
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

      <div class="consumption-cost-summary">
        <span>当前查询条件下的产品消耗成本</span>
        <strong>{{ currency(totalConsumptionCost) }}</strong>
        <small>共 {{ filteredConsumptions.length }} 条消耗明细</small>
      </div>

      <el-table
        :data="consumptionSummary"
        row-key="productId"
        stripe
        class="salon-table material-table"
        empty-text="当前条件下暂无产品消耗"
      >
        <el-table-column prop="productName" label="产品名称" min-width="240" />
        <el-table-column label="消耗" min-width="160">
          <template #default="{ row }">
            <strong class="consumption-value">{{ row.quantity }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="消耗成本" min-width="160">
          <template #default="{ row }">
            <strong class="cost-value">{{ currency(row.cost) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="130" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="openDetails(row.productId, row.productName)">
              查看明细
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <BaseModal
      v-if="detailVisible"
      title="消耗明细"
      :subtitle="detailSubtitle"
      wide
      @close="detailVisible = false"
    >
      <el-table
        :data="currentResultDetails"
        row-key="id"
        stripe
        max-height="480"
        empty-text="当前搜索结果中没有消耗明细"
      >
        <el-table-column label="消耗时间" min-width="165">
          <template #default="{ row }">{{ fullDateTime(row.consumedAt) }}</template>
        </el-table-column>
        <el-table-column prop="memberName" label="顾客" min-width="120" />
        <el-table-column prop="employee" label="服务员工" min-width="110" />
        <el-table-column label="来源" min-width="110">
          <template #default="{ row }">
            {{ row.sourceType === 'project' ? '单次项目' : '套餐' }}
          </template>
        </el-table-column>
        <el-table-column prop="quantity" label="消耗数量" min-width="100" />
        <el-table-column label="单位成本" min-width="110">
          <template #default="{ row }">{{ currency(row.unitCost) }}</template>
        </el-table-column>
        <el-table-column label="消耗成本" min-width="110">
          <template #default="{ row }">{{ currency(row.quantity * row.unitCost) }}</template>
        </el-table-column>
      </el-table>
    </BaseModal>
  </div>
</template>

<style scoped>
.material-page {
  display: block;
}

.material-panel {
  overflow: hidden;
}

.consumption-filter {
  display: grid;
  grid-template-columns: minmax(200px, 280px) minmax(340px, 1fr) auto;
  gap: 16px;
  align-items: end;
  margin: 16px 16px 20px;
  padding: 16px;
  background: #faf8f7;
  border: 1px solid #eee8e5;
  border-radius: 12px;
}

.filter-field {
  display: grid;
  gap: 7px;
}

.filter-field > span {
  color: var(--muted);
  font-size: 13px;
  font-weight: 600;
}

.date-field :deep(.el-date-editor) {
  width: 100%;
}

.filter-actions {
  display: flex;
  gap: 8px;
}

.consumption-cost-summary {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin: 0 16px 16px;
  padding: 14px 16px;
  color: var(--muted);
  background: #fff7f4;
  border: 1px solid #f2ded7;
  border-radius: 12px;
}

.consumption-cost-summary strong {
  color: var(--rose);
  font-size: 22px;
}

.consumption-cost-summary small {
  margin-left: auto;
}

.consumption-value {
  color: var(--rose);
  font-size: 16px;
}

.cost-value {
  color: #8d4f3f;
}

@media (max-width: 900px) {
  .consumption-filter {
    grid-template-columns: 1fr;
  }

  .filter-actions .el-button {
    flex: 1;
  }

  .consumption-cost-summary {
    align-items: flex-start;
    flex-direction: column;
  }

  .consumption-cost-summary small {
    margin-left: 0;
  }
}
</style>
