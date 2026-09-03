<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  ArrowDownRight,
  ArrowUpRight,
  Banknote,
  CalendarDays,
  CircleDollarSign,
  Download,
  Layers3,
  PackageOpen,
  TrendingUp,
  UsersRound
} from 'lucide-vue-next'
import StatCard from '../components/StatCard.vue'
import RevenueLineChart from '../components/RevenueLineChart.vue'
import TablePagination from '../components/TablePagination.vue'
import { useTablePagination } from '../composables/useTablePagination'
import { REPORT_RANGES, type ReportRange } from '../config/options'
import { salonStore } from '../data/repository'
import {
  buildAmountTrend,
  buildEmployeeStats,
  buildRevenueTrend,
  recordsInRange,
  type AmountTrendRecord,
  type TrendAggregation
} from '../services/reporting'
import { currency, dateFromDaysAgo, downloadCsv, isToday } from '../utils'
import { notify } from '../utils/feedback'

const range = ref<ReportRange>('day')
type RevenueMetric =
  'actualIncome' | 'recharge' | 'boxPackage' | 'normalPackage' | 'serviceAverage' | 'consumption'

const selectedMetric = ref<RevenueMetric>('actualIncome')
const rangeLabel = computed(() => REPORT_RANGES[range.value].label)
const transactions = computed(() => recordsInRange(salonStore.transactions, range.value))
const services = computed(() =>
  recordsInRange(salonStore.services, range.value).filter(item => item.status === 'completed')
)
const consumption = computed(() =>
  transactions.value
    .filter(item => item.type === 'consume' && item.status !== 'cancelled')
    .reduce((sum, item) => sum + item.amount, 0)
)
const recharge = computed(() =>
  transactions.value
    .filter(item => item.type === 'recharge')
    .reduce((sum, item) => sum + item.amount, 0)
)
const packagePurchases = computed(() => {
  const cutoff = dateFromDaysAgo(REPORT_RANGES[range.value].days)
  return salonStore.packagePurchases.filter(item => new Date(item.purchasedAt) >= cutoff)
})
const boxPackageSales = computed(() =>
  packagePurchases.value
    .filter(item => item.packageType === '套盒')
    .reduce((sum, item) => sum + item.price, 0)
)
const normalPackageSales = computed(() =>
  packagePurchases.value
    .filter(item => item.packageType === '普通')
    .reduce((sum, item) => sum + item.price, 0)
)
const average = computed(() =>
  services.value.length
    ? services.value.reduce((sum, item) => sum + item.amount, 0) / services.value.length
    : 0
)
const todayRechargeIncome = computed(() =>
  salonStore.transactions
    .filter(
      item => item.type === 'recharge' && item.status !== 'cancelled' && isToday(item.createdAt)
    )
    .reduce((sum, item) => sum + item.amount, 0)
)
const todayServiceIncome = computed(() =>
  salonStore.services
    .filter(item => item.status === 'completed' && isToday(item.createdAt))
    .reduce((sum, item) => sum + (item.externalPaymentAmount ?? 0), 0)
)
const todayPackageIncome = computed(() =>
  salonStore.packagePurchases
    .filter(item => isToday(item.purchasedAt))
    .reduce((sum, item) => sum + item.cashPaymentAmount, 0)
)
const todayLegacyIncome = computed(() =>
  salonStore.transactions
    .filter(
      item =>
        item.type === 'consume' &&
        item.status !== 'cancelled' &&
        isToday(item.createdAt) &&
        (!item.sourceType || item.sourceType === 'legacy') &&
        item.paymentMethod !== '会员余额'
    )
    .reduce((sum, item) => sum + item.amount, 0)
)
const todayRevenue = computed(
  () =>
    todayRechargeIncome.value +
    todayServiceIncome.value +
    todayPackageIncome.value +
    todayLegacyIncome.value
)
const todayRevenueHint = computed(() => {
  const parts = [
    `充值 ${currency(todayRechargeIncome.value)}`,
    `服务实付 ${currency(todayServiceIncome.value)}`,
    `套餐现金 ${currency(todayPackageIncome.value)}`
  ]
  if (todayLegacyIncome.value) parts.push(`其他入账 ${currency(todayLegacyIncome.value)}`)
  return parts.join(' · ')
})

const actualIncomeRecords = computed<AmountTrendRecord[]>(() => [
  ...salonStore.transactions
    .filter(item => item.type === 'recharge' && item.status !== 'cancelled')
    .map(item => ({ createdAt: item.createdAt, amount: item.amount })),
  ...salonStore.services
    .filter(item => item.status === 'completed' && (item.externalPaymentAmount ?? 0) > 0)
    .map(item => ({ createdAt: item.createdAt, amount: item.externalPaymentAmount ?? 0 })),
  ...salonStore.packagePurchases
    .filter(item => item.cashPaymentAmount > 0)
    .map(item => ({ createdAt: item.purchasedAt, amount: item.cashPaymentAmount })),
  ...salonStore.transactions
    .filter(
      item =>
        item.type === 'consume' &&
        item.status !== 'cancelled' &&
        (!item.sourceType || item.sourceType === 'legacy') &&
        item.paymentMethod !== '会员余额'
    )
    .map(item => ({ createdAt: item.createdAt, amount: item.amount }))
])
const metricMeta: Record<
  RevenueMetric,
  { label: string; description: string; aggregation?: TrendAggregation }
> = {
  actualIncome: {
    label: '实际入账',
    description: '充值本金、服务外部支付与套餐现金支付合计'
  },
  recharge: { label: '会员充值', description: '会员实充本金，不包含赠送金额' },
  boxPackage: { label: '套盒套餐销售', description: '套盒类型套餐成交金额' },
  normalPackage: { label: '普通套餐销售', description: '普通类型套餐成交金额' },
  serviceAverage: {
    label: '服务客单价',
    description: '各时间段内已完成服务的平均金额',
    aggregation: 'average'
  },
  consumption: { label: '消费营业额', description: '普通消费与套餐销售成交金额' }
}
const selectedMetricMeta = computed(() => metricMeta[selectedMetric.value])
const selectedMetricRecords = computed<AmountTrendRecord[]>(() => {
  switch (selectedMetric.value) {
    case 'actualIncome':
      return actualIncomeRecords.value
    case 'recharge':
      return salonStore.transactions
        .filter(item => item.type === 'recharge' && item.status !== 'cancelled')
        .map(item => ({ createdAt: item.createdAt, amount: item.amount }))
    case 'boxPackage':
      return salonStore.packagePurchases
        .filter(item => item.packageType === '套盒')
        .map(item => ({ createdAt: item.purchasedAt, amount: item.price }))
    case 'normalPackage':
      return salonStore.packagePurchases
        .filter(item => item.packageType === '普通')
        .map(item => ({ createdAt: item.purchasedAt, amount: item.price }))
    case 'serviceAverage':
      return salonStore.services
        .filter(item => item.status === 'completed')
        .map(item => ({ createdAt: item.createdAt, amount: item.amount }))
    default:
      return []
  }
})
const trend = computed(() =>
  selectedMetric.value === 'consumption'
    ? buildRevenueTrend(salonStore.transactions, range.value)
    : buildAmountTrend(
        selectedMetricRecords.value,
        range.value,
        selectedMetricMeta.value.aggregation
      )
)

const employeeStats = computed(() => buildEmployeeStats(salonStore.employees, services.value))
const {
  currentPage: employeePage,
  pageSize: employeePageSize,
  paginatedRecords: paginatedEmployeeStats
} = useTablePagination(employeeStats)

function selectMetric(metric: RevenueMetric) {
  selectedMetric.value = metric
}

function exportReport() {
  const rows = [
    ['员工', '服务次数', '服务金额'],
    ...employeeStats.value.map(item => [item.name, item.count, item.revenue])
  ]
  downloadCsv(`营收报表-${rangeLabel.value}.csv`, rows)
  notify('营收报表已导出')
}
</script>

<template>
  <div class="page">
    <section class="section-toolbar">
      <el-radio-group v-model="range" class="salon-radio-group">
        <el-radio-button v-for="(item, key) in REPORT_RANGES" :key="key" :value="key">
          {{ item.label }}
        </el-radio-button>
      </el-radio-group>
      <el-button @click="exportReport">
        <Download :size="17" />
        导出报表
      </el-button>
    </section>
    <section class="stat-grid">
      <StatCard
        class="revenue-metric-card"
        :class="{ 'is-selected': selectedMetric === 'actualIncome' }"
        role="button"
        tabindex="0"
        :aria-pressed="selectedMetric === 'actualIncome'"
        label="今日营收"
        :value="currency(todayRevenue)"
        :hint="todayRevenueHint"
        trend="今日实际入账"
        :icon="CircleDollarSign"
        tone="rose"
        @click="selectMetric('actualIncome')"
        @keydown.enter="selectMetric('actualIncome')"
        @keydown.space.prevent="selectMetric('actualIncome')"
      />
      <StatCard
        class="revenue-metric-card"
        :class="{ 'is-selected': selectedMetric === 'recharge' }"
        role="button"
        tabindex="0"
        :aria-pressed="selectedMetric === 'recharge'"
        label="会员充值"
        :value="currency(recharge)"
        :hint="`${transactions.filter(t => t.type === 'recharge').length} 笔充值入账`"
        trend="储值资金"
        :icon="Banknote"
        tone="gold"
        @click="selectMetric('recharge')"
        @keydown.enter="selectMetric('recharge')"
        @keydown.space.prevent="selectMetric('recharge')"
      />
      <StatCard
        class="revenue-metric-card"
        :class="{ 'is-selected': selectedMetric === 'boxPackage' }"
        role="button"
        tabindex="0"
        :aria-pressed="selectedMetric === 'boxPackage'"
        label="套盒套餐销售"
        :value="currency(boxPackageSales)"
        :hint="`${packagePurchases.filter(item => item.packageType === '套盒').length} 笔销售`"
        trend="已包含在消费营业额"
        :icon="PackageOpen"
        tone="violet"
        @click="selectMetric('boxPackage')"
        @keydown.enter="selectMetric('boxPackage')"
        @keydown.space.prevent="selectMetric('boxPackage')"
      />
      <StatCard
        class="revenue-metric-card"
        :class="{ 'is-selected': selectedMetric === 'normalPackage' }"
        role="button"
        tabindex="0"
        :aria-pressed="selectedMetric === 'normalPackage'"
        label="普通套餐销售"
        :value="currency(normalPackageSales)"
        :hint="`${packagePurchases.filter(item => item.packageType === '普通').length} 笔销售`"
        trend="已包含在消费营业额"
        :icon="Layers3"
        tone="gold"
        @click="selectMetric('normalPackage')"
        @keydown.enter="selectMetric('normalPackage')"
        @keydown.space.prevent="selectMetric('normalPackage')"
      />
      <StatCard
        class="revenue-metric-card"
        :class="{ 'is-selected': selectedMetric === 'serviceAverage' }"
        role="button"
        tabindex="0"
        :aria-pressed="selectedMetric === 'serviceAverage'"
        label="服务客单价"
        :value="currency(average)"
        :hint="`${services.length} 次服务`"
        :icon="UsersRound"
        tone="violet"
        @click="selectMetric('serviceAverage')"
        @keydown.enter="selectMetric('serviceAverage')"
        @keydown.space.prevent="selectMetric('serviceAverage')"
      />
      <StatCard
        class="revenue-metric-card"
        :class="{ 'is-selected': selectedMetric === 'consumption' }"
        role="button"
        tabindex="0"
        :aria-pressed="selectedMetric === 'consumption'"
        label="消费营业额"
        :value="currency(consumption)"
        :hint="`${transactions.filter(t => t.type === 'consume' && t.status !== 'cancelled').length} 笔消费流水`"
        trend="成交口径"
        :icon="TrendingUp"
        tone="rose"
        @click="selectMetric('consumption')"
        @keydown.enter="selectMetric('consumption')"
        @keydown.space.prevent="selectMetric('consumption')"
      />
    </section>
    <section class="panel full-chart-panel">
      <header class="panel-head">
        <div>
          <h3>{{ rangeLabel }}{{ selectedMetricMeta.label }}曲线</h3>
          <p>{{ selectedMetricMeta.description }}</p>
        </div>
        <span class="date-chip">
          <CalendarDays :size="15" />
          {{ rangeLabel }}
        </span>
      </header>
      <div class="large-chart">
        <RevenueLineChart :data="trend" :name="selectedMetricMeta.label" />
      </div>
    </section>
    <section class="panel table-panel employee-report">
      <header class="panel-head">
        <div>
          <h3>员工业绩明细</h3>
          <p>{{ rangeLabel }}服务产出统计</p>
        </div>
      </header>
      <el-table :data="paginatedEmployeeStats" row-key="id" stripe class="salon-table">
        <el-table-column label="排名" width="72">
          <template #default="{ $index }">
            <span
              class="rank"
              :class="`rank-${(employeePage - 1) * employeePageSize + $index + 1}`"
            >
              {{ (employeePage - 1) * employeePageSize + $index + 1 }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="员工" min-width="130">
          <template #default="{ row }">
            <div class="employee-inline">
              <span :style="{ background: row.color }">{{ row.name.slice(-1) }}</span>
              <b>{{ row.name }}</b>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="role" label="岗位" min-width="120" />
        <el-table-column label="服务次数" width="100">
          <template #default="{ row }">{{ row.count }} 次</template>
        </el-table-column>
        <el-table-column label="服务金额" width="115">
          <template #default="{ row }">
            <strong class="money">{{ currency(row.revenue) }}</strong>
          </template>
        </el-table-column>
        <el-table-column label="业绩趋势" width="110">
          <template #default="{ row }">
            <span class="trend-up" v-if="row.revenue">
              <ArrowUpRight :size="15" />
              表现良好
            </span>
            <span class="trend-flat" v-else>
              <ArrowDownRight :size="15" />
              暂无服务
            </span>
          </template>
        </el-table-column>
      </el-table>
      <TablePagination
        v-model="employeePage"
        v-model:page-size="employeePageSize"
        :total="employeeStats.length"
      />
    </section>
  </div>
</template>

<style scoped>
.revenue-metric-card {
  cursor: pointer;
  transition:
    transform 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.revenue-metric-card:hover {
  transform: translateY(-2px);
}

.revenue-metric-card.is-selected {
  border-color: var(--color-primary, #d8657e);
  box-shadow: 0 0 0 2px rgb(216 101 126 / 12%);
}

.revenue-metric-card:focus-visible {
  outline: 2px solid var(--color-primary, #d8657e);
  outline-offset: 2px;
}
</style>
