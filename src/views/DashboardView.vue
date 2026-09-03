<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  ArrowRight,
  CalendarDays,
  CreditCard,
  ListTree,
  ReceiptText,
  Sparkles,
  TrendingUp,
  UserPlus,
  UsersRound
} from 'lucide-vue-next'
import StatCard from '../components/StatCard.vue'
import RevenueLineChart from '../components/RevenueLineChart.vue'
import { authStore } from '../auth'
import { salonStore } from '../data/repository'
import { APPOINTMENT_STATUS_OPTIONS } from '../config/options'
import { currency, dateTime, fullDateTime, isToday } from '../utils'

const router = useRouter()
const todayTransactions = computed(() =>
  salonStore.transactions.filter(item => isToday(item.createdAt))
)
const todayServices = computed(() =>
  salonStore.services.filter(item => isToday(item.createdAt) && item.status === 'completed')
)
const isManager = computed(() => authStore.user?.role === 'manager')
const storeServices = computed(() => todayServices.value)
const storeServiceAmount = computed(() =>
  storeServices.value.reduce((sum, item) => sum + item.amount, 0)
)
const todayRevenue = computed(() =>
  todayTransactions.value
    .filter(item => item.type === 'consume' && item.status !== 'cancelled')
    .reduce((sum, item) => sum + item.amount, 0)
)
const todayRecharge = computed(() =>
  todayTransactions.value
    .filter(item => item.type === 'recharge')
    .reduce((sum, item) => sum + item.amount, 0)
)
const totalBalance = computed(() =>
  salonStore.members.reduce((sum, member) => sum + member.balance, 0)
)
const todayAppointments = computed(() =>
  salonStore.appointments
    .filter(
      item => isToday(item.startsAt) && item.status !== 'cancelled' && item.status !== 'no_show'
    )
    .sort((a, b) => a.startsAt.localeCompare(b.startsAt))
)
const appointmentStatusLabel = (status: (typeof salonStore.appointments)[number]['status']) =>
  APPOINTMENT_STATUS_OPTIONS.find(item => item.value === status)?.label ?? status

const trend = computed(() => {
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date()
    date.setDate(date.getDate() - (6 - index))
    const amount = salonStore.transactions
      .filter(item => {
        const current = new Date(item.createdAt)
        return (
          item.type === 'consume' &&
          item.status !== 'cancelled' &&
          current.toDateString() === date.toDateString()
        )
      })
      .reduce((sum, item) => sum + item.amount, 0)
    return { label: `${date.getMonth() + 1}/${date.getDate()}`, amount }
  })
  return days
})

const employeeRanking = computed(() =>
  salonStore.employees
    .map(employee => {
      const records = todayServices.value.filter(item => item.employee === employee.name)
      return {
        ...employee,
        count: records.length,
        amount: records.reduce((sum, item) => sum + item.amount, 0)
      }
    })
    .sort((a, b) => b.amount - a.amount)
)
</script>

<template>
  <div class="page dashboard-page">
    <section class="welcome-strip">
      <div>
        <span>{{ isManager ? '今日经营提示' : '我的工作台' }}</span>
        <h2>你好，{{ authStore.user?.displayName }}，今天也要元气满满 ✨</h2>
        <p v-if="isManager">
          今日已有 {{ todayServices.length }} 位顾客到店，会员储值余额共
          {{ currency(totalBalance) }}
        </p>
        <p v-else>今日门店已完成 {{ storeServices.length }} 次服务，可继续登记新的到店服务。</p>
      </div>
    </section>

    <section v-if="isManager" class="stat-grid">
      <StatCard
        label="今日消费营业额"
        :value="currency(todayRevenue)"
        hint="已完成消费成交金额"
        trend="较昨日 +12.6%"
        :icon="TrendingUp"
        tone="rose"
      />
      <StatCard
        label="今日充值"
        :value="currency(todayRecharge)"
        hint="会员储值入账"
        trend="资金流水可追溯"
        :icon="CreditCard"
        tone="gold"
      />
      <StatCard
        label="服务人次"
        :value="`${todayServices.length} 人`"
        :hint="`共 ${todayServices.reduce((sum, item) => sum + item.duration, 0)} 分钟`"
        :icon="Sparkles"
        tone="violet"
      />
      <StatCard
        label="会员总数"
        :value="`${salonStore.members.length} 人`"
        hint="活跃会员档案"
        :icon="UsersRound"
        tone="green"
      />
    </section>

    <section v-else class="stat-grid employee-stat-grid">
      <StatCard
        label="今日门店服务"
        :value="`${storeServices.length} 次`"
        hint="全店今日已完成记录"
        :icon="Sparkles"
        tone="rose"
      />
      <StatCard
        label="服务金额"
        :value="currency(storeServiceAmount)"
        hint="全店今日服务合计"
        :icon="CreditCard"
        tone="gold"
      />
      <StatCard
        label="会员总数"
        :value="`${salonStore.members.length} 人`"
        hint="可查询会员档案"
        :icon="UsersRound"
        tone="green"
      />
    </section>

    <section class="panel dashboard-appointment-panel">
      <header class="panel-head">
        <div>
          <h3>今日预约</h3>
          <p>共 {{ todayAppointments.length }} 项有效预约</p>
        </div>
        <div class="element-row-actions">
          <el-button size="small" @click="router.push('/appointments?action=new')">
            新增预约
          </el-button>
          <el-button size="small" @click="router.push('/appointments')">
            全部预约
            <ArrowRight :size="15" />
          </el-button>
        </div>
      </header>
      <div v-if="todayAppointments.length" class="activity-list">
        <div v-for="item in todayAppointments.slice(0, 5)" :key="item.id" class="activity-row">
          <span class="activity-icon consume"><CalendarDays :size="18" /></span>
          <div>
            <b>{{ item.customerName }} · {{ item.serviceName }}</b>
            <small>{{ item.employee }} · {{ fullDateTime(item.startsAt) }}</small>
          </div>
          <el-tag round>{{ appointmentStatusLabel(item.status) }}</el-tag>
        </div>
      </div>
      <div v-else class="empty-state compact">
        <CalendarDays :size="26" />
        <b>今天还没有预约</b>
        <span>可以登记会员或游客预约</span>
      </div>
    </section>

    <section v-if="isManager" class="dashboard-grid">
      <article class="panel revenue-panel">
        <header class="panel-head">
          <div>
            <h3>近 7 日营收趋势</h3>
            <p>按消费营业额统计</p>
          </div>
          <el-button size="small" @click="router.push('/revenue')">
            查看完整报表
            <ArrowRight :size="15" />
          </el-button>
        </header>
        <div class="mini-chart">
          <RevenueLineChart :data="trend" />
        </div>
      </article>

      <article class="panel ranking-panel">
        <header class="panel-head">
          <div>
            <h3>今日员工业绩</h3>
            <p>服务金额实时排行</p>
          </div>
        </header>
        <div class="ranking-list">
          <div v-for="(employee, index) in employeeRanking" :key="employee.id" class="ranking-row">
            <span class="rank" :class="`rank-${index + 1}`">{{ index + 1 }}</span>
            <span class="employee-dot" :style="{ background: employee.color }">
              {{ employee.name.slice(-1) }}
            </span>
            <div>
              <b>{{ employee.name }}</b>
              <small>{{ employee.count }} 次服务</small>
            </div>
            <strong>{{ currency(employee.amount) }}</strong>
          </div>
        </div>
      </article>
    </section>

    <section v-if="isManager" class="dashboard-grid lower-grid">
      <article class="panel">
        <header class="panel-head">
          <div>
            <h3>最近资金流水</h3>
            <p>充值与消费动态</p>
          </div>
          <el-button size="small" @click="router.push('/members?tab=transactions')">
            全部流水
            <ArrowRight :size="15" />
          </el-button>
        </header>
        <div class="activity-list">
          <div
            v-for="item in salonStore.transactions.slice(0, 4)"
            :key="item.id"
            class="activity-row"
          >
            <span class="activity-icon" :class="item.type"><ReceiptText :size="18" /></span>
            <div>
              <b>{{ item.memberName }} · {{ item.item }}</b>
              <small>{{ item.employee }} · {{ dateTime(item.createdAt) }}</small>
            </div>
            <strong :class="item.type">
              {{ item.type === 'recharge' ? '+' : '-' }}{{ currency(item.amount) }}
            </strong>
          </div>
        </div>
      </article>
      <article class="panel quick-panel">
        <header class="panel-head">
          <div>
            <h3>快捷操作</h3>
            <p>常用功能一步直达</p>
          </div>
        </header>
        <div class="quick-grid">
          <el-button class="quick-action" @click="router.push('/members?action=new')">
            <span class="quick-action-icon"><UserPlus :size="21" /></span>
            <b>新增会员</b>
            <small>建立会员档案</small>
          </el-button>
          <el-button class="quick-action" @click="router.push('/projects')">
            <span class="quick-action-icon"><ListTree :size="21" /></span>
            <b>项目管理</b>
            <small>维护项目价格时长</small>
          </el-button>
          <el-button class="quick-action" @click="router.push('/revenue')">
            <span class="quick-action-icon"><TrendingUp :size="21" /></span>
            <b>营收报表</b>
            <small>查看经营走势</small>
          </el-button>
        </div>
      </article>
    </section>

    <section v-else class="dashboard-grid employee-dashboard-grid">
      <article class="panel">
        <header class="panel-head">
          <div>
            <h3>今日门店服务</h3>
            <p>员工共用账号可查看全店服务记录</p>
          </div>
          <el-button size="small" @click="router.push('/services')">
            全部记录
            <ArrowRight :size="15" />
          </el-button>
        </header>
        <div v-if="storeServices.length" class="activity-list">
          <div v-for="item in storeServices.slice(0, 5)" :key="item.id" class="activity-row">
            <span class="activity-icon consume"><Sparkles :size="18" /></span>
            <div>
              <b>{{ item.memberName }} · {{ item.serviceName }}</b>
              <small>{{ item.duration }} 分钟 · {{ dateTime(item.createdAt) }}</small>
            </div>
            <strong>{{ currency(item.amount) }}</strong>
          </div>
        </div>
        <div v-else class="empty-state compact">
          <Sparkles :size="26" />
          <b>今天还没有服务记录</b>
          <span>完成服务后请及时登记</span>
        </div>
      </article>
      <article class="panel quick-panel">
        <header class="panel-head">
          <div>
            <h3>快捷操作</h3>
            <p>员工常用功能一步直达</p>
          </div>
        </header>
        <div class="quick-grid">
          <el-button class="quick-action" @click="router.push('/members?action=new')">
            <span class="quick-action-icon"><UserPlus :size="21" /></span>
            <b>新增会员</b>
            <small>建立会员档案</small>
          </el-button>
          <el-button class="quick-action" @click="router.push('/projects')">
            <span class="quick-action-icon"><ListTree :size="21" /></span>
            <b>项目管理</b>
            <small>维护项目价格时长</small>
          </el-button>
          <el-button class="quick-action" @click="router.push('/profile')">
            <span class="quick-action-icon"><UserPlus :size="21" /></span>
            <b>我的账号</b>
            <small>修改登录密码</small>
          </el-button>
        </div>
      </article>
    </section>
  </div>
</template>
