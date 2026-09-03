import { STORAGE_KEYS } from '../config/app'
import { COMMISSION_RULE_VERSION } from '../config/options'
import type {
  AppSnapshot,
  EmployeeInput,
  MemberInput,
  ServiceInput,
  TransactionInput
} from '../types'
import { seedData } from './seed'
import {
  appendBrowserEmployeeStatusEvent,
  normalizeBusinessSnapshot
} from './browserBusinessRepository'
import { normalizeAppointments } from './browserAppointmentRepository'

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value))

export function loadBrowserSnapshot(): AppSnapshot {
  const cached = localStorage.getItem(STORAGE_KEYS.salonData)
  const snapshot: AppSnapshot = cached ? JSON.parse(cached) : clone(seedData)
  normalizeBusinessSnapshot(snapshot)
  normalizeAppointments(snapshot)
  saveBrowserSnapshot(snapshot)
  return snapshot
}

export function saveBrowserSnapshot(snapshot: AppSnapshot) {
  localStorage.setItem(STORAGE_KEYS.salonData, JSON.stringify(snapshot))
}

export function addBrowserMember(state: AppSnapshot, input: MemberInput) {
  if (input.giftAmount > 0 && input.initialBalance <= 0)
    throw new Error('赠送金额必须与开卡充值同时登记')
  const employee = state.employees.find(
    item => item.name === input.employee && item.status === 'active'
  )
  if (!employee) throw new Error('请选择有效的经办员工')
  const id = crypto.randomUUID()
  const now = new Date().toISOString()
  const creditedAmount = input.initialBalance + input.giftAmount
  state.members.unshift({
    id,
    name: input.name,
    phone: input.phone,
    balance: creditedAmount,
    principalBalance: input.initialBalance,
    giftBalance: input.giftAmount,
    totalRecharge: input.initialBalance,
    totalConsumption: 0,
    joinDate: now,
    lastVisit: now,
    status: 'active'
  })
  if (input.initialBalance > 0) {
    state.transactions.unshift({
      id: crypto.randomUUID(),
      memberId: id,
      memberName: input.name,
      type: 'recharge',
      amount: input.initialBalance,
      giftAmount: input.giftAmount,
      commission:
        Math.round(
          input.initialBalance *
            (state.employeeCompensations.find(item => item.employeeId === employee.id)
              ?.baseCommissionRate ?? 0.1) *
            100
        ) / 100,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: creditedAmount,
      paymentMethod: '现金',
      item: '开卡充值',
      employee: employee.name,
      createdAt: now,
      note: '新会员开卡'
    })
  }
}

export function addBrowserTransaction(state: AppSnapshot, input: TransactionInput) {
  if (input.type !== 'recharge' || input.paymentMethod === '会员余额')
    throw new Error('会员管理仅支持充值，消费请通过登记消费完成')
  const member = state.members.find(item => item.id === input.memberId)
  const employee = state.employees.find(
    item => item.name === input.employee && item.status === 'active'
  )
  if (!member) throw new Error('没有找到该会员')
  if (!employee) throw new Error('请选择有效的经办员工')
  const giftAmount = input.giftAmount
  member.principalBalance = Math.round((member.principalBalance + input.amount) * 100) / 100
  member.giftBalance = Math.round((member.giftBalance + giftAmount) * 100) / 100
  member.totalRecharge += input.amount
  member.balance = Math.round((member.principalBalance + member.giftBalance) * 100) / 100
  member.lastVisit = new Date().toISOString()
  state.transactions.unshift({
    id: crypto.randomUUID(),
    memberId: member.id,
    memberName: member.name,
    type: input.type,
    amount: input.amount,
    giftAmount,
    commission:
      Math.round(
        input.amount *
          (state.employeeCompensations.find(item => item.employeeId === employee.id)
            ?.baseCommissionRate ?? 0.1) *
          100
      ) / 100,
    commissionRuleVersion: COMMISSION_RULE_VERSION,
    balanceAfter: member.balance,
    paymentMethod: input.paymentMethod,
    item: input.item,
    employee: employee.name,
    createdAt: new Date().toISOString(),
    note: input.note
  })
}

export function addBrowserService(state: AppSnapshot, input: ServiceInput) {
  if (state.services.some(item => item.id === input.requestId))
    throw new Error('该服务已经登记，请勿重复提交')
  const member = state.members.find(item => item.id === input.memberId)
  const project = state.projects.find(
    item => item.id === input.projectId && item.status === 'active'
  )
  const employee = state.employees.find(
    item => item.name === input.employee && item.status === 'active'
  )
  if ((!member && !input.guestName.trim()) || !employee || !project)
    throw new Error('顾客、员工或项目信息不完整')
  const validExternalMethods = ['微信支付', '支付宝', '现金', '银行卡']
  const balancePaymentAmount = member ? Math.min(member.balance, project.price) : 0
  const externalPaymentAmount = Math.round((project.price - balancePaymentAmount) * 100) / 100
  if (externalPaymentAmount > 0 && !validExternalMethods.includes(input.externalPaymentMethod))
    throw new Error('请选择实际支付方式')
  const giftDeduction = member ? Math.min(member.giftBalance, balancePaymentAmount) : 0
  const principalDeduction = balancePaymentAmount - giftDeduction
  if (member) {
    member.giftBalance = Math.round((member.giftBalance - giftDeduction) * 100) / 100
    member.principalBalance = Math.round((member.principalBalance - principalDeduction) * 100) / 100
    member.balance = Math.round((member.principalBalance + member.giftBalance) * 100) / 100
    member.totalConsumption += project.price
  }
  const paymentMethod =
    balancePaymentAmount > 0 && externalPaymentAmount > 0
      ? `会员余额+${input.externalPaymentMethod}`
      : balancePaymentAmount > 0
        ? '会员余额'
        : input.externalPaymentMethod
  const now = new Date().toISOString()
  const transactionId = crypto.randomUUID()
  state.services.unshift({
    id: input.requestId,
    memberId: member?.id ?? null,
    memberName: member?.name ?? input.guestName.trim(),
    employee: employee.name,
    serviceName: project.name,
    serviceType: '普通手工',
    duration: project.duration,
    amount: project.price,
    commission:
      state.employeeCompensations.find(item => item.employeeId === employee.id)
        ?.normalServiceCommission ?? 5,
    commissionRuleVersion: COMMISSION_RULE_VERSION,
    packagePurchaseId: null,
    projectId: project.id,
    balancePaymentAmount,
    externalPaymentAmount,
    paymentMethod,
    giftDeduction,
    principalDeduction,
    transactionId,
    createdAt: now,
    status: 'completed'
  })
  state.transactions.unshift({
    id: transactionId,
    memberId: member?.id ?? null,
    memberName: member?.name ?? input.guestName.trim(),
    type: 'consume',
    amount: project.price,
    giftAmount: 0,
    commission: 0,
    commissionRuleVersion: COMMISSION_RULE_VERSION,
    balanceAfter: member?.balance ?? 0,
    paymentMethod,
    item: project.name,
    employee: employee.name,
    createdAt: now,
    note: '普通消费自动结算',
    status: 'active',
    sourceType: 'service',
    sourceId: input.requestId
  })
  if (member) member.lastVisit = now
}

export function cancelBrowserService(state: AppSnapshot, serviceId: string) {
  const service = state.services.find(
    item =>
      item.id === serviceId &&
      item.status === 'completed' &&
      item.serviceType === '普通手工' &&
      item.transactionId
  )
  if (!service) throw new Error('该服务不存在、已撤销或不支持撤销')
  const member = service.memberId
    ? state.members.find(item => item.id === service.memberId)
    : undefined
  if (service.memberId && !member) throw new Error('会员不存在，无法撤销')
  if (member) {
    member.giftBalance = Math.round((member.giftBalance + (service.giftDeduction ?? 0)) * 100) / 100
    member.principalBalance =
      Math.round((member.principalBalance + (service.principalDeduction ?? 0)) * 100) / 100
    member.balance = Math.round((member.principalBalance + member.giftBalance) * 100) / 100
    member.totalConsumption = Math.max(0, member.totalConsumption - service.amount)
  }
  service.status = 'cancelled'
  const transaction = state.transactions.find(item => item.id === service.transactionId)
  if (transaction) {
    transaction.status = 'cancelled'
    transaction.note = `${transaction.note}；服务已撤销`
  }
}

export function addBrowserEmployee(state: AppSnapshot, input: EmployeeInput) {
  if (state.employees.some(item => item.name === input.name)) throw new Error('员工姓名已存在')
  const {
    baseSalary,
    baseCommissionRate,
    performanceTarget,
    excessCommissionRate,
    mealAllowancePerDay,
    attendanceBonus,
    normalServiceCommission,
    packageServiceCommission,
    ...profile
  } = input
  const employee = {
    id: crypto.randomUUID(),
    ...profile,
    status: 'active' as const,
    createdAt: new Date().toISOString()
  }
  state.employees.push(employee)
  state.employeeCompensations.push({
    employeeId: employee.id,
    baseSalary,
    baseCommissionRate,
    performanceTarget,
    excessCommissionRate,
    mealAllowancePerDay,
    attendanceBonus,
    normalServiceCommission,
    packageServiceCommission,
    createdAt: employee.createdAt
  })
  appendBrowserEmployeeStatusEvent(state, employee.id, 'active')
}

export function setBrowserEmployeeStatus(
  state: AppSnapshot,
  employeeId: string,
  status: 'active' | 'inactive'
) {
  const employee = state.employees.find(item => item.id === employeeId)
  if (!employee) throw new Error('员工不存在')
  employee.status = status
  appendBrowserEmployeeStatusEvent(state, employeeId, status)
}

export function downloadBrowserBackup(state: AppSnapshot) {
  const blob = new Blob([JSON.stringify(state, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `聚尚木子数据备份-${new Date().toISOString().slice(0, 10)}.json`
  link.click()
  URL.revokeObjectURL(url)
}
