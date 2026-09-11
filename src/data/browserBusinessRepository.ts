import {
  COMMISSION_RULE_VERSION,
  DEFAULT_COMMISSION_CONFIG,
  DEFAULT_EMPLOYEE_COMPENSATION,
  DEFAULT_ATTENDANCE_REST_DAYS,
  PACKAGE_COMMISSION_RULE_VERSION
} from '../config/options'
import type {
  AppSnapshot,
  AttendanceInput,
  CommissionConfig,
  CommissionConfigInput,
  EmployeeStatusEvent,
  PackageConsumptionInput,
  PackageDefinitionInput,
  PackagePurchaseInput,
  ProjectDefinitionInput
} from '../types'
import { isPackagePurchaseAvailable, localMonthKey } from '../utils'

const roundMoney = (value: number) => Math.round(value * 100) / 100

function transactionBalanceDeduction(
  snapshot: AppSnapshot,
  record: AppSnapshot['transactions'][number]
) {
  if (record.type !== 'consume') return 0
  if (record.item.startsWith('套盒购买：') || record.item.startsWith('套餐购买：')) {
    const purchase = snapshot.packagePurchases.find(item => item.transactionId === record.id)
    if (purchase?.balancePaymentAmount !== undefined) return purchase.balancePaymentAmount
  }
  return record.paymentMethod === '会员余额' ? record.amount : 0
}

function legacyBalanceParts(snapshot: AppSnapshot, memberId: string, currentBalance: number) {
  const records = snapshot.transactions
    .filter(item => item.memberId === memberId)
    .sort((a, b) => a.createdAt.localeCompare(b.createdAt) || a.id.localeCompare(b.id))
  if (!records.length) return { principalBalance: currentBalance, giftBalance: 0 }
  const first = records[0]
  let principalBalance =
    first.type === 'recharge'
      ? first.balanceAfter - first.amount - (first.giftAmount ?? 0)
      : first.balanceAfter + transactionBalanceDeduction(snapshot, first)
  principalBalance = Math.max(0, roundMoney(principalBalance))
  let giftBalance = 0

  records.forEach(record => {
    if (record.type === 'recharge') {
      principalBalance = roundMoney(principalBalance + record.amount)
      giftBalance = roundMoney(giftBalance + (record.giftAmount ?? 0))
      return
    }
    if (record.item.startsWith('套盒购买：') || record.item.startsWith('套餐购买：')) {
      principalBalance = roundMoney(
        Math.max(0, principalBalance - transactionBalanceDeduction(snapshot, record))
      )
      return
    }
    if (record.paymentMethod !== '会员余额') return
    const giftDeduction = Math.min(giftBalance, record.amount)
    giftBalance = roundMoney(giftBalance - giftDeduction)
    principalBalance = roundMoney(Math.max(0, principalBalance - (record.amount - giftDeduction)))
  })

  const difference = roundMoney(currentBalance - principalBalance - giftBalance)
  if (difference >= 0) principalBalance = roundMoney(principalBalance + difference)
  else {
    const deficit = -difference
    const giftReduction = Math.min(giftBalance, deficit)
    giftBalance = roundMoney(giftBalance - giftReduction)
    principalBalance = roundMoney(Math.max(0, principalBalance - (deficit - giftReduction)))
  }
  return { principalBalance, giftBalance }
}

export function currentCommissionConfig(snapshot: AppSnapshot, month = localMonthKey()) {
  return [...snapshot.commissionConfigs]
    .filter(item => item.effectiveMonth <= month)
    .sort((a, b) => b.effectiveMonth.localeCompare(a.effectiveMonth))[0]
}

function commissionConfigAt(snapshot: AppSnapshot, createdAt: string) {
  const month = localMonthKey(createdAt) || localMonthKey()
  return (
    currentCommissionConfig(snapshot, month) ??
    [...snapshot.commissionConfigs].sort((a, b) =>
      a.effectiveMonth.localeCompare(b.effectiveMonth)
    )[0]
  )
}

export function normalizeBusinessSnapshot(snapshot: AppSnapshot) {
  const now = new Date().toISOString()
  snapshot.employeeStatusEvents ??= []
  snapshot.employeeCompensations ??= []
  snapshot.commissionConfigs ??= []
  snapshot.attendanceRecords ??= []
  snapshot.packages ??= []
  snapshot.packagePurchases ??= []
  snapshot.packageConsumptions ??= []
  snapshot.projects ??= []

  if (!snapshot.commissionConfigs.length) {
    snapshot.commissionConfigs.push({
      id: crypto.randomUUID(),
      effectiveMonth: '1970-01',
      ...DEFAULT_COMMISSION_CONFIG,
      createdAt: now
    })
  }

  snapshot.packagePurchases.forEach(purchase => {
    const packageDefinition = snapshot.packages.find(item => item.id === purchase.packageId)
    purchase.packageType ??= packageDefinition?.packageType ?? '套盒'
    purchase.limitType ??= packageDefinition?.limitType ?? 'count'
    purchase.validityDays ??= packageDefinition?.validityDays ?? 0
    purchase.activatedAt ??= purchase.purchasedAt
    purchase.expiresAt ??=
      purchase.limitType === 'time' && purchase.validityDays > 0
        ? new Date(
            new Date(purchase.activatedAt ?? purchase.purchasedAt).getTime() +
              purchase.validityDays * 86_400_000
          ).toISOString()
        : null
    if (
      purchase.limitType === 'time' &&
      purchase.expiresAt &&
      new Date(purchase.expiresAt).getTime() <= Date.now()
    )
      purchase.status = 'completed'
    purchase.balancePaymentAmount ??= purchase.paymentMethod === '会员余额' ? purchase.price : 0
    purchase.cashPaymentAmount ??= purchase.paymentMethod === '现金' ? purchase.price : 0
  })

  snapshot.packages.forEach(item => {
    item.packageType ??= '套盒'
    item.limitType ??= 'count'
    item.validityDays ??= 0
  })

  snapshot.members.forEach(member => {
    delete (member as typeof member & { level?: unknown }).level
    if (member.principalBalance === undefined || member.giftBalance === undefined) {
      Object.assign(member, legacyBalanceParts(snapshot, member.id, member.balance))
    }
    member.balance = roundMoney(member.principalBalance + member.giftBalance)
  })

  snapshot.employees.forEach(employee => {
    employee.createdAt ??=
      [...snapshot.services, ...snapshot.transactions]
        .filter(item => item.employee === employee.name)
        .map(item => item.createdAt)
        .sort()[0] ?? now
    if (!snapshot.employeeStatusEvents.some(item => item.employeeId === employee.id)) {
      snapshot.employeeStatusEvents.push({
        id: crypto.randomUUID(),
        employeeId: employee.id,
        status: 'active',
        changedAt: employee.createdAt
      })
      if (employee.status === 'inactive') {
        snapshot.employeeStatusEvents.push({
          id: crypto.randomUUID(),
          employeeId: employee.id,
          status: 'inactive',
          changedAt: now
        })
      }
    }
    if (!snapshot.employeeCompensations.some(item => item.employeeId === employee.id)) {
      snapshot.employeeCompensations.push({
        employeeId: employee.id,
        ...DEFAULT_EMPLOYEE_COMPENSATION,
        createdAt: employee.createdAt
      })
    }
  })

  snapshot.transactions.forEach(transaction => {
    transaction.giftAmount ??= 0
    transaction.status ??= 'active'
    transaction.sourceType ??= snapshot.packagePurchases.some(
      item => item.transactionId === transaction.id
    )
      ? 'package_purchase'
      : 'legacy'
    transaction.sourceId ??=
      snapshot.packagePurchases.find(item => item.transactionId === transaction.id)?.id ?? null
    if ((transaction.commissionRuleVersion ?? 0) < COMMISSION_RULE_VERSION) {
      const config = commissionConfigAt(snapshot, transaction.createdAt)
      transaction.commission =
        transaction.type === 'recharge' ? roundMoney(transaction.amount * config.rechargeRate) : 0
      transaction.commissionRuleVersion = COMMISSION_RULE_VERSION
    }
  })
  snapshot.services.forEach(service => {
    service.serviceType ??= '普通手工'
    service.packagePurchaseId ??= null
    service.projectId ??= null
    service.balancePaymentAmount ??= 0
    service.externalPaymentAmount ??= 0
    service.paymentMethod ??= ''
    service.giftDeduction ??= 0
    service.principalDeduction ??= 0
    service.transactionId ??= null
    if ((service.commissionRuleVersion ?? 0) < COMMISSION_RULE_VERSION) {
      const config = commissionConfigAt(snapshot, service.createdAt)
      if (service.packagePurchaseId) {
        service.commission ??= config.packageServiceAmount
      } else {
        service.commission =
          service.serviceType === '套盒手工'
            ? config.packageServiceAmount
            : config.normalServiceAmount
      }
      service.commissionRuleVersion = COMMISSION_RULE_VERSION
    }
  })
  snapshot.packagePurchases.forEach(purchase => {
    if ((purchase.commissionRuleVersion ?? 0) < PACKAGE_COMMISSION_RULE_VERSION) {
      const employee = snapshot.employees.find(item => item.name === purchase.employee)
      const compensation = snapshot.employeeCompensations.find(
        item => item.employeeId === employee?.id
      )
      purchase.commission = roundMoney(
        purchase.cashPaymentAmount *
          (compensation?.baseCommissionRate ?? DEFAULT_EMPLOYEE_COMPENSATION.baseCommissionRate)
      )
      purchase.commissionRuleVersion = PACKAGE_COMMISSION_RULE_VERSION
    }
  })
}

export function createBrowserProject(snapshot: AppSnapshot, input: ProjectDefinitionInput) {
  const name = input.name.trim()
  if (!name || input.duration <= 0 || input.price <= 0) throw new Error('项目信息无效')
  if (snapshot.projects.some(item => item.name === name)) throw new Error('项目名称已存在')
  const now = new Date().toISOString()
  snapshot.projects.unshift({
    id: crypto.randomUUID(),
    name,
    duration: input.duration,
    price: roundMoney(input.price),
    status: 'active',
    createdAt: now,
    updatedAt: now
  })
}

export function updateBrowserProject(
  snapshot: AppSnapshot,
  projectId: string,
  input: ProjectDefinitionInput
) {
  const project = snapshot.projects.find(item => item.id === projectId)
  if (!project) throw new Error('项目不存在')
  const name = input.name.trim()
  if (!name || input.duration <= 0 || input.price <= 0) throw new Error('项目信息无效')
  if (snapshot.projects.some(item => item.id !== projectId && item.name === name))
    throw new Error('项目名称已存在')
  Object.assign(project, {
    name,
    duration: input.duration,
    price: roundMoney(input.price),
    updatedAt: new Date().toISOString()
  })
}

export function setBrowserProjectStatus(
  snapshot: AppSnapshot,
  projectId: string,
  status: 'active' | 'inactive'
) {
  const project = snapshot.projects.find(item => item.id === projectId)
  if (!project) throw new Error('项目不存在')
  project.status = status
  project.updatedAt = new Date().toISOString()
}

export function saveBrowserCommissionConfig(snapshot: AppSnapshot, input: CommissionConfigInput) {
  const existing = snapshot.commissionConfigs.find(
    item => item.effectiveMonth === input.effectiveMonth
  )
  const config: CommissionConfig = {
    id: existing?.id ?? crypto.randomUUID(),
    ...input,
    createdAt: existing?.createdAt ?? new Date().toISOString()
  }
  if (existing) Object.assign(existing, config)
  else snapshot.commissionConfigs.push(config)
}

export function saveBrowserAttendance(snapshot: AppSnapshot, input: AttendanceInput) {
  const employee = snapshot.employees.find(item => item.id === input.employeeId)
  if (!employee) throw new Error('员工不存在')
  const existing = snapshot.attendanceRecords.find(
    item => item.employeeId === input.employeeId && item.month === input.month
  )
  const now = new Date().toISOString()
  if (existing) Object.assign(existing, { restDays: input.restDays, updatedAt: now })
  else {
    snapshot.attendanceRecords.push({
      id: crypto.randomUUID(),
      ...input,
      updatedAt: now
    })
  }
}

export function createBrowserPackage(snapshot: AppSnapshot, input: PackageDefinitionInput) {
  const name = input.name.trim()
  if (
    !name ||
    !Number.isFinite(input.price) ||
    input.price < 0 ||
    !['count', 'time'].includes(input.limitType) ||
    (input.limitType === 'count' && (!Number.isInteger(input.totalUses) || input.totalUses <= 0)) ||
    (input.limitType === 'time' &&
      (!Number.isInteger(input.validityDays) || input.validityDays <= 0))
  )
    throw new Error('套餐名称、价格或可用次数无效')
  if (snapshot.packages.some(item => item.name === name)) throw new Error('套餐名称已存在')
  const now = new Date().toISOString()
  snapshot.packages.unshift({
    id: crypto.randomUUID(),
    name,
    price: roundMoney(input.price),
    totalUses: input.limitType === 'count' ? input.totalUses : 1,
    limitType: input.limitType,
    validityDays: input.limitType === 'time' ? input.validityDays : 0,
    packageType: input.packageType,
    status: 'active',
    createdAt: now,
    updatedAt: now
  })
}

export function updateBrowserPackage(
  snapshot: AppSnapshot,
  packageId: string,
  input: PackageDefinitionInput
) {
  const packageItem = snapshot.packages.find(item => item.id === packageId)
  if (!packageItem) throw new Error('套餐不存在')
  const name = input.name.trim()
  if (
    !name ||
    !Number.isFinite(input.price) ||
    input.price < 0 ||
    !['count', 'time'].includes(input.limitType) ||
    (input.limitType === 'count' && (!Number.isInteger(input.totalUses) || input.totalUses <= 0)) ||
    (input.limitType === 'time' &&
      (!Number.isInteger(input.validityDays) || input.validityDays <= 0))
  )
    throw new Error('套餐名称、价格或可用次数无效')
  if (snapshot.packages.some(item => item.id !== packageId && item.name === name))
    throw new Error('套餐名称已存在')
  Object.assign(packageItem, {
    ...input,
    name,
    price: roundMoney(input.price),
    totalUses: input.limitType === 'count' ? input.totalUses : 1,
    validityDays: input.limitType === 'time' ? input.validityDays : 0,
    updatedAt: new Date().toISOString()
  })
}

export function setBrowserPackageStatus(
  snapshot: AppSnapshot,
  packageId: string,
  status: 'active' | 'inactive'
) {
  const packageItem = snapshot.packages.find(item => item.id === packageId)
  if (!packageItem) throw new Error('套餐不存在')
  packageItem.status = status
  packageItem.updatedAt = new Date().toISOString()
}

export function purchaseBrowserPackage(snapshot: AppSnapshot, input: PackagePurchaseInput) {
  const member = snapshot.members.find(item => item.id === input.memberId)
  const packageItem = snapshot.packages.find(
    item => item.id === input.packageId && item.status === 'active'
  )
  const employee = snapshot.employees.find(
    item => item.name === input.employee && item.status === 'active'
  )
  if (!member || !packageItem || !employee) throw new Error('会员、套餐或销售员工信息无效')
  let balancePaymentAmount = 0
  let cashPaymentAmount = 0
  if (input.paymentMethod === '会员余额') balancePaymentAmount = packageItem.price
  else if (input.paymentMethod === '现金') cashPaymentAmount = packageItem.price
  else if (input.paymentMethod === '余额现金组合支付') {
    balancePaymentAmount = roundMoney(input.balancePaymentAmount)
    cashPaymentAmount = roundMoney(input.cashPaymentAmount)
    if (!Number.isFinite(balancePaymentAmount) || !Number.isFinite(cashPaymentAmount))
      throw new Error('支付金额无效')
    if (balancePaymentAmount <= 0 || cashPaymentAmount <= 0)
      throw new Error('组合支付的余额支付和现金支付金额都必须大于0')
    const total = roundMoney(balancePaymentAmount + cashPaymentAmount)
    if (total < packageItem.price)
      throw new Error(`支付总金额不足套餐价格，还差${roundMoney(packageItem.price - total)}元`)
    if (total > packageItem.price) throw new Error('支付总金额不能超过套餐价格')
  } else throw new Error('套餐支付方式无效')
  if (member.principalBalance < balancePaymentAmount)
    throw new Error('实付本金余额不足，赠送余额不可购买套餐')

  const nowValue = new Date()
  const now = nowValue.toISOString()
  const activatedAtValue = new Date(input.activatedAt)
  if (Number.isNaN(activatedAtValue.getTime()) || activatedAtValue.getTime() > nowValue.getTime())
    throw new Error('开卡时间无效或晚于当前时间')
  const activatedAt = activatedAtValue.toISOString()
  const expiresAt =
    packageItem.limitType === 'time'
      ? new Date(activatedAtValue.getTime() + packageItem.validityDays * 86_400_000).toISOString()
      : null
  const transactionId = packageItem.price > 0 ? crypto.randomUUID() : ''
  const purchaseId = crypto.randomUUID()
  member.principalBalance = roundMoney(member.principalBalance - balancePaymentAmount)
  member.balance = roundMoney(member.principalBalance + member.giftBalance)
  member.totalConsumption += packageItem.price
  member.lastVisit = now
  if (packageItem.price > 0) {
    snapshot.transactions.unshift({
      id: transactionId,
      memberId: member.id,
      memberName: member.name,
      type: 'consume',
      amount: packageItem.price,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: member.balance,
      paymentMethod: input.paymentMethod,
      item: `套餐购买：${packageItem.name}`,
      employee: employee.name,
      createdAt: now,
      note: '',
      status: 'active',
      sourceType: 'package_purchase',
      sourceId: purchaseId
    })
  }
  const compensation = snapshot.employeeCompensations.find(item => item.employeeId === employee.id)
  snapshot.packagePurchases.unshift({
    id: purchaseId,
    memberId: member.id,
    memberName: member.name,
    packageId: packageItem.id,
    packageName: packageItem.name,
    packageType: packageItem.packageType,
    employee: employee.name,
    price: packageItem.price,
    totalUses: packageItem.totalUses,
    remainingUses: packageItem.totalUses,
    limitType: packageItem.limitType,
    validityDays: packageItem.validityDays,
    activatedAt,
    expiresAt,
    paymentMethod: input.paymentMethod,
    balancePaymentAmount,
    cashPaymentAmount,
    commission: roundMoney(
      cashPaymentAmount *
        (compensation?.baseCommissionRate ?? DEFAULT_EMPLOYEE_COMPENSATION.baseCommissionRate)
    ),
    commissionRuleVersion: PACKAGE_COMMISSION_RULE_VERSION,
    purchasedAt: now,
    lastConsumedAt: null,
    status:
      expiresAt && new Date(expiresAt).getTime() <= nowValue.getTime() ? 'completed' : 'active',
    transactionId
  })
}

export function consumeBrowserPackage(snapshot: AppSnapshot, input: PackageConsumptionInput) {
  const purchase = snapshot.packagePurchases.find(item => item.id === input.purchaseId)
  const employee = snapshot.employees.find(
    item => item.name === input.employee && item.status === 'active'
  )
  if (!purchase || !isPackagePurchaseAvailable(purchase))
    throw new Error('该套餐已结束、已过期或剩余次数不足')
  if (!employee) throw new Error('服务员工无效')
  const now = new Date().toISOString()
  const serviceId = crypto.randomUUID()
  if (purchase.limitType === 'count') purchase.remainingUses -= 1
  purchase.lastConsumedAt = now
  if (purchase.limitType === 'count' && purchase.remainingUses === 0) purchase.status = 'completed'
  const compensation = snapshot.employeeCompensations.find(item => item.employeeId === employee.id)
  const serviceType = purchase.packageType === '普通' ? '普通手工' : '套盒手工'
  const commission =
    serviceType === '普通手工'
      ? (compensation?.normalServiceCommission ?? 5)
      : (compensation?.packageServiceCommission ?? 10)
  snapshot.services.unshift({
    id: serviceId,
    memberId: purchase.memberId,
    memberName: purchase.memberName,
    employee: employee.name,
    serviceName: purchase.packageName,
    serviceType,
    duration: input.duration,
    amount: 0,
    commission,
    commissionRuleVersion: COMMISSION_RULE_VERSION,
    packagePurchaseId: purchase.id,
    createdAt: now,
    status: 'completed'
  })
  snapshot.packageConsumptions.unshift({
    id: crypto.randomUUID(),
    purchaseId: purchase.id,
    employee: employee.name,
    duration: input.duration,
    remainingAfter: purchase.remainingUses,
    createdAt: now,
    note: input.note.trim(),
    serviceId
  })
  const member = snapshot.members.find(item => item.id === purchase.memberId)
  if (member) member.lastVisit = now
}

export function appendBrowserEmployeeStatusEvent(
  snapshot: AppSnapshot,
  employeeId: string,
  status: EmployeeStatusEvent['status']
) {
  snapshot.employeeStatusEvents.push({
    id: crypto.randomUUID(),
    employeeId,
    status,
    changedAt: new Date().toISOString()
  })
}

export function attendanceRestDays(snapshot: AppSnapshot, employeeId: string, month: string) {
  return (
    snapshot.attendanceRecords.find(item => item.employeeId === employeeId && item.month === month)
      ?.restDays ?? DEFAULT_ATTENDANCE_REST_DAYS
  )
}
