import { DEFAULT_EMPLOYEE_COMPENSATION } from '../config/options'
import type {
  AppSnapshot,
  CommissionEntry,
  CommissionSettings,
  PrincipalAllocation
} from '../types'
import { localMonthKey, serviceEmployeeNames } from '../utils'

const money = (value: number) => {
  const cents = Math.floor(Math.abs(value) * 100 + 0.5 + 1e-8)
  return cents ? (Math.sign(value) * cents) / 100 : 0
}
type Lot = PrincipalAllocation & { remaining: number; remainingCommission: number }
type Event = {
  type: CommissionEntry['sourceType']
  id: string
  time: string
  priority: number
}

export function validateCommissionSettings(input: CommissionSettings) {
  if (
    !['general', 'independent'].includes(input.commissionMode) ||
    !Number.isFinite(input.commissionRate) ||
    input.commissionRate < 0 ||
    input.commissionRate > 1
  )
    throw new Error('请选择有效的提成方式，独立提成比例必须在0%至100%之间')
}

/** Immutable sales entries; principal is consumed FIFO, including ordinary spending. */
export function synchronizeCommissions(snapshot: AppSnapshot, legacy = false) {
  snapshot.commissionEntries ??= []
  const saved = new Map(snapshot.commissionEntries.map(item => [item.id, item]))
  const processed: CommissionEntry[] = []
  const lots = new Map<string, Lot[]>()
  const gifts = new Map<string, number>()
  const events: Event[] = []
  const add = (type: Event['type'], id: string, time: string, priority: number) =>
    events.push({ type, id, time, priority })
  for (const item of snapshot.transactions) {
    if (item.type === 'recharge') add('recharge', item.id, item.createdAt, 0)
    else if (
      !snapshot.services.some(service => service.transactionId === item.id) &&
      !snapshot.packagePurchases.some(purchase => purchase.transactionId === item.id) &&
      item.status !== 'cancelled'
    )
      add('legacy', item.id, item.createdAt, 2)
  }
  for (const item of snapshot.packagePurchases) add('package', item.id, item.purchasedAt, 1)
  for (const item of snapshot.services) {
    if (item.packagePurchaseId) continue
    add('service', item.id, item.createdAt, 2)
    if (item.status === 'cancelled') add('cancel', item.id, item.cancelledAt ?? item.createdAt, 4)
  }
  for (const item of snapshot.refundRecords) add('refund', item.id, item.createdAt, 3)
  events.sort(
    (a, b) =>
      Date.parse(a.time) - Date.parse(b.time) || a.priority - b.priority || a.id.localeCompare(b.id)
  )

  const wallet = (memberId: string) => {
    if (!lots.has(memberId)) lots.set(memberId, [])
    return lots.get(memberId)!
  }
  const take = (
    memberId: string,
    amount: number,
    known?: PrincipalAllocation[]
  ): PrincipalAllocation[] => {
    if (known) {
      for (const allocation of known) {
        let remaining = allocation.amount
        let credit = allocation.commission
        for (const lot of wallet(memberId).filter(
          item => item.rechargeId === allocation.rechargeId
        )) {
          const used = money(Math.min(remaining, lot.remaining))
          const usedCredit =
            used === remaining
              ? credit
              : Math.min(credit, money((used * allocation.commission) / allocation.amount))
          lot.remaining = money(lot.remaining - used)
          lot.remainingCommission = money(Math.max(0, lot.remainingCommission - usedCredit))
          remaining = money(remaining - used)
          credit = money(credit - usedCredit)
          if (remaining <= 0) break
        }
      }
      return known
    }
    let remaining = money(amount)
    const result: PrincipalAllocation[] = []
    for (const lot of wallet(memberId)) {
      if (remaining <= 0) break
      const used = money(Math.min(remaining, lot.remaining))
      if (used <= 0) continue
      const credit =
        used === lot.remaining
          ? lot.remainingCommission
          : Math.min(lot.remainingCommission, money((lot.commission * used) / lot.amount))
      result.push({
        rechargeId: lot.rechargeId,
        amount: used,
        commission: credit,
        verified: lot.verified
      })
      lot.remaining = money(lot.remaining - used)
      lot.remainingCommission = money(lot.remainingCommission - credit)
      remaining = money(remaining - used)
    }
    if (remaining > 0)
      result.push({ rechargeId: '', amount: remaining, commission: 0, verified: false })
    return result
  }
  const giveBack = (memberId: string, allocations: PrincipalAllocation[], amount: number) => {
    const total = money(allocations.reduce((sum, item) => sum + item.amount, 0))
    let remaining = money(Math.min(total, amount))
    for (let index = 0; index < allocations.length; index++) {
      const item = allocations[index]!
      const restored =
        index === allocations.length - 1
          ? remaining
          : Math.min(remaining, money((amount * item.amount) / total))
      remaining = money(remaining - restored)
      if (restored <= 0) continue
      const credit =
        restored === item.amount
          ? item.commission
          : money((item.commission * restored) / item.amount)
      const originalLot = wallet(memberId).find(lot => lot.rechargeId === item.rechargeId)
      if (originalLot) {
        originalLot.remaining = money(originalLot.remaining + restored)
        originalLot.remainingCommission = money(originalLot.remainingCommission + credit)
      } else
        wallet(memberId).push({
          ...item,
          amount: restored,
          commission: credit,
          remaining: restored,
          remainingCommission: credit
        })
    }
  }

  for (const event of events) {
    const key = `${event.type}:${event.id}`
    let entry = saved.get(key)
    const transaction = snapshot.transactions.find(item => item.id === event.id)
    const purchase = snapshot.packagePurchases.find(item => item.id === event.id)
    const service = snapshot.services.find(item => item.id === event.id)
    const refund = snapshot.refundRecords.find(item => item.id === event.id)
    const source =
      event.type === 'recharge' || event.type === 'legacy'
        ? transaction!
        : event.type === 'package'
          ? purchase!
          : event.type === 'refund'
            ? refund!
            : service!
    const memberId = source.memberId
    let amount = 0,
      balance = 0,
      cash = 0,
      performance = 0
    let mode: CommissionEntry['mode'] = 'general',
      rate = 0
    let principal: PrincipalAllocation[] = []
    let employeeNames: string[] = []
    let name = ''
    if (event.type === 'recharge') {
      amount = transaction!.amount
      cash = amount
      performance = amount
      employeeNames = [transaction!.employee]
      name = transaction!.item || '会员充值'
      if (memberId) gifts.set(memberId, money((gifts.get(memberId) ?? 0) + transaction!.giftAmount))
    } else if (event.type === 'package') {
      amount = purchase!.price
      balance = purchase!.balancePaymentAmount
      cash = purchase!.cashPaymentAmount
      mode = purchase!.commissionMode ?? 'general'
      rate = purchase!.commissionRate ?? 0
      performance = mode === 'general' ? cash : 0
      employeeNames = [purchase!.employee]
      name = purchase!.packageName
      if (memberId) principal = take(memberId, balance, entry?.principalAllocations)
    } else if (event.type === 'service') {
      amount = service!.amount
      balance = service!.principalDeduction ?? 0
      cash = service!.externalPaymentAmount ?? 0
      mode = service!.commissionMode ?? 'general'
      rate = service!.commissionRate ?? 0
      employeeNames = serviceEmployeeNames(service!)
      name = service!.serviceName
      if (memberId) {
        principal = take(memberId, balance, entry?.principalAllocations)
        gifts.set(
          memberId,
          money(Math.max(0, (gifts.get(memberId) ?? 0) - (service!.giftDeduction ?? 0)))
        )
      }
    } else if (event.type === 'legacy') {
      amount = transaction!.amount
      name = transaction!.item
      if (memberId && transaction!.paymentMethod.includes('会员余额')) {
        const packagePurchase = name.startsWith('套盒购买：') || name.startsWith('套餐购买：')
        const giftDeduction = packagePurchase ? 0 : Math.min(gifts.get(memberId) ?? 0, amount)
        balance = entry?.balanceAmount ?? money(amount - giftDeduction)
        principal = take(memberId, balance, entry?.principalAllocations)
        gifts.set(memberId, money((gifts.get(memberId) ?? 0) - giftDeduction))
      }
    } else if (event.type === 'cancel') {
      const original =
        saved.get(`service:${event.id}`) ??
        processed.find(item => item.id === `service:${event.id}`)
      if (original && memberId)
        giveBack(memberId, original.principalAllocations, original.balanceAmount)
      if (memberId)
        gifts.set(memberId, money((gifts.get(memberId) ?? 0) + (service!.giftDeduction ?? 0)))
      mode = original?.mode ?? 'general'
      rate = original?.rate ?? 0
      amount = -service!.amount
      balance = -(original?.balanceAmount ?? 0)
      cash = -(original?.cashAmount ?? 0)
      name = `撤销：${service!.serviceName}`
    } else {
      amount = -refund!.amount
      name = refund!.refundType === 'account' ? '会员账户退款' : '套餐退款'
      employeeNames = [refund!.employee]
      if (refund!.refundType === 'account') {
        performance = -refund!.cashAmount
        cash = -refund!.cashAmount
        if (memberId) {
          const giftBefore = gifts.get(memberId) ?? 0
          balance = refund!.cashAmount + Math.max(0, refund!.giftForfeitedAmount - giftBefore)
          principal = take(memberId, entry?.balanceAmount ?? balance, entry?.principalAllocations)
          gifts.set(memberId, Math.max(0, money(giftBefore - refund!.giftForfeitedAmount)))
        }
      } else {
        const original =
          saved.get(`package:${refund!.packagePurchaseId}`) ??
          processed.find(item => item.id === `package:${refund!.packagePurchaseId}`)
        if (original && memberId)
          giveBack(memberId, original.principalAllocations, refund!.balanceAmount)
        mode = original?.mode ?? 'general'
        rate = original?.rate ?? 0
        balance = -refund!.balanceAmount
        cash = -refund!.cashAmount
        performance = mode === 'general' ? -refund!.cashAmount : 0
      }
    }

    if (!entry) {
      if (
        !legacy &&
        mode === 'independent' &&
        principal.some(item => !item.verified && item.amount > 0)
      )
        throw new Error(
          '账户本金的历史充值来源不完整，无法确认已计提成。请核对充值记录或改用额外付款。'
        )
      let commission = 0,
        offset = 0,
        balanceCommission = 0,
        cashCommission = 0
      let employees: CommissionEntry['employees'] = []
      const employeeIds = [
        ...new Set(
          employeeNames
            .map(employeeName => snapshot.employees.find(item => item.name === employeeName)?.id)
            .filter((id): id is string => !!id)
        )
      ]
      if (mode === 'independent' && (event.type === 'package' || event.type === 'service')) {
        offset = money(principal.reduce((sum, item) => sum + item.commission, 0))
        balanceCommission = money(
          principal.reduce(
            (sum, item) => sum + Math.max(0, money(item.amount * rate) - item.commission),
            0
          )
        )
        cashCommission = money(cash * rate)
        commission = money(balanceCommission + cashCommission)
      } else if (event.type === 'refund' && mode === 'independent') {
        const original = saved.get(`package:${refund!.packagePurchaseId}`)!
        commission =
          original.amount > 0 ? -money((original.commission * refund!.amount) / original.amount) : 0
        offset =
          original.amount > 0
            ? -money((original.rechargeOffset * refund!.amount) / original.amount)
            : 0
        balanceCommission =
          original.amount > 0
            ? -money((original.balanceCommission * refund!.amount) / original.amount)
            : 0
        cashCommission = money(commission - balanceCommission)
        let remaining = commission
        employees = original.employees.map((item, index) => {
          const share =
            index === original.employees.length - 1
              ? remaining
              : original.commission
                ? -money((item.commission * -commission) / original.commission)
                : 0
          remaining = money(remaining - share)
          return { ...item, commission: share }
        })
      } else if (event.type === 'cancel' && mode === 'independent') {
        const original = saved.get(`service:${event.id}`)!
        commission = -original.commission
        balanceCommission = -original.balanceCommission
        cashCommission = -original.cashCommission
        offset = -original.rechargeOffset
        employees = original.employees.map(item => ({ ...item, commission: -item.commission }))
      } else if (performance && employeeIds[0]) {
        const compensation =
          snapshot.employeeCompensations.find(item => item.employeeId === employeeIds[0]) ??
          DEFAULT_EMPLOYEE_COMPENSATION
        const previous = money(
          processed
            .filter(
              item =>
                item.mode === 'general' &&
                item.employees.some(employee => employee.employeeId === employeeIds[0]) &&
                localMonthKey(item.createdAt) === localMonthKey(event.time)
            )
            .reduce((sum, item) => sum + item.performanceAmount, 0)
        )
        const cumulative = (value: number) =>
          money(
            value * compensation.baseCommissionRate +
              Math.max(0, value - compensation.performanceTarget) *
                compensation.excessCommissionRate
          )
        commission = money(cumulative(previous + performance) - cumulative(previous))
        rate = performance ? commission / performance : 0
        cashCommission = commission
      }
      if (!employees.length) {
        let remaining = commission
        employees = employeeIds.map((employeeId, index) => {
          const share =
            index === employeeIds.length - 1 ? remaining : money(commission / employeeIds.length)
          remaining = money(remaining - share)
          return { employeeId, commission: share }
        })
      }
      entry = {
        id: key,
        sourceType: event.type,
        sourceId: event.id,
        memberId,
        memberName: source.memberName,
        businessName: name,
        mode,
        rate,
        amount,
        balanceAmount: balance,
        cashAmount: cash,
        performanceAmount: performance,
        rechargeOffset: offset,
        balanceCommission,
        cashCommission,
        commission,
        principalAllocations: principal,
        employees,
        createdAt: event.time,
        legacy
      }
      snapshot.commissionEntries.push(entry)
      saved.set(key, entry)
    }
    processed.push(entry)
    if (event.type === 'recharge' && memberId)
      wallet(memberId).push({
        rechargeId: event.id,
        amount,
        commission: entry.commission,
        remaining: amount,
        remainingCommission: entry.commission,
        verified: true
      })
  }
}

export function independentCommissionForEmployee(
  entry: { employees: ReadonlyArray<{ employeeId: string; commission: number }> },
  employeeId: string
) {
  return entry.employees.find(item => item.employeeId === employeeId)?.commission ?? 0
}
