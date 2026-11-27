import test, { type TestContext } from 'node:test'
import assert from 'node:assert/strict'
import { seedData } from '../src/data/seed'
import { DEFAULT_EMPLOYEE_COMPENSATION } from '../src/config/options'
import {
  addBrowserEmployee,
  addBrowserMember,
  addBrowserTransaction,
  addBrowserService,
  cancelBrowserService
} from '../src/data/browserRepository'
import {
  createBrowserPackage,
  purchaseBrowserPackage,
  refundBrowserPackage,
  consumeBrowserPackage,
  normalizeBusinessSnapshot,
  createBrowserProject,
  updateBrowserPackage
} from '../src/data/browserBusinessRepository'
import {
  buildSalaryRows,
  buildHandworkDetails,
  buildPerformanceDetails
} from '../src/services/payroll'
import { isPackagePurchaseAvailable } from '../src/utils/packageLimit'
import type { AppSnapshot } from '../src/types'

function setup(context: TestContext) {
  context.mock.timers.enable({ apis: ['Date'], now: Date.parse('2026-11-01T01:00:00Z') })
  const snapshot: AppSnapshot = structuredClone(seedData)
  const step = (mutation: () => void) => {
    context.mock.timers.tick(1000)
    mutation()
    normalizeBusinessSnapshot(snapshot)
  }
  for (const name of ['A', 'B'])
    step(() =>
      addBrowserEmployee(snapshot, {
        ...DEFAULT_EMPLOYEE_COMPENSATION,
        name,
        role: '美容师',
        color: '#cc6278'
      })
    )
  const recharge = (amount: number, employee = 'A', giftAmount = 0) =>
    step(() => {
      if (!snapshot.members.length)
        addBrowserMember(snapshot, {
          name: '顾客',
          phone: '13800000000',
          initialBalance: amount,
          giftAmount,
          employee
        })
      else
        addBrowserTransaction(snapshot, {
          memberId: snapshot.members[0]!.id,
          amount,
          employee,
          giftAmount,
          type: 'recharge',
          paymentMethod: '现金',
          item: '会员充值',
          note: ''
        })
    })
  const definePackage = (price: number, rate = 0.2, extra = {}) => {
    step(() =>
      createBrowserPackage(snapshot, {
        name: `套餐${snapshot.packages.length}`,
        price,
        totalUses: 12,
        limitType: 'count',
        validityDays: 365,
        packageType: '普通',
        productId: null,
        consumptionQuantity: 0,
        commissionMode: 'independent',
        commissionRate: rate,
        giftPackageId: null,
        giftUses: 0,
        ...extra
      })
    )
    return snapshot.packages[0]!
  }
  const purchase = (packageId: string, balance: number, cash: number, employee = 'B') => {
    step(() =>
      purchaseBrowserPackage(snapshot, {
        packageId,
        memberId: snapshot.members[0]!.id,
        employee,
        activatedAt: new Date().toISOString(),
        paymentMethod: balance && cash ? '余额现金组合支付' : balance ? '会员余额' : '现金',
        balancePaymentAmount: balance,
        cashPaymentAmount: cash,
        note: ''
      })
    )
    return snapshot.packagePurchases.find(
      item => item.packageId === packageId && !item.parentPurchaseId
    )!
  }
  const salary = (name: string, month = '2026-11') =>
    buildSalaryRows(snapshot, month).find(item => item.name === name)!
  return { snapshot, step, recharge, definePackage, purchase, salary, context }
}

test('A充值1000，B用990余额售卡，A100、B99；余额剩10，重复同步不重复计提', context => {
  const s = setup(context)
  s.recharge(1000)
  const card = s.definePackage(990)
  const purchase = s.purchase(card.id, 990, 0)
  assert.equal(s.salary('A').generalCommission, 100)
  assert.equal(s.salary('B').independentCommission, 99)
  assert.equal(s.snapshot.members[0]!.principalBalance, 10)
  const count = s.snapshot.commissionEntries.length
  normalizeBusinessSnapshot(s.snapshot)
  assert.equal(s.snapshot.commissionEntries.length, count)
  assert.equal(
    s.snapshot.commissionEntries.find(item => item.sourceId === purchase.id)!.rechargeOffset,
    99
  )
})

test('余额1000加现金980购买1980年卡，B补差100加现金196，现金不计通用绩效', context => {
  const s = setup(context)
  s.recharge(1000)
  const card = s.definePackage(1980)
  s.purchase(card.id, 1000, 980)
  assert.equal(s.salary('A').generalCommission, 100)
  assert.equal(s.salary('B').independentCommission, 296)
  assert.equal(s.salary('B').totalPerformance, 0)
  assert.equal(buildPerformanceDetails(s.snapshot, s.salary('B').employeeId, '2026-11').length, 0)
})

test('多次充值不同员工比例，按本金FIFO抵扣；已消费本金不能再次用于补差', context => {
  const s = setup(context)
  s.recharge(1000)
  s.snapshot.employeeCompensations.find(
    item => item.employeeId === s.snapshot.employees.find(item => item.name === 'B')!.id
  )!.baseCommissionRate = 0.12
  s.recharge(1000, 'B')
  const general = s.definePackage(600, 0, { commissionMode: 'general' })
  s.purchase(general.id, 600, 0)
  const water = s.definePackage(990)
  const purchase = s.purchase(water.id, 990, 0)
  const entry = s.snapshot.commissionEntries.find(item => item.sourceId === purchase.id)!
  assert.equal(entry.rechargeOffset, 110.8)
  assert.equal(entry.commission, 87.2)
})

test('超目标奖励纳入充值已计提金额；后续改比例不改历史结果', context => {
  const s = setup(context)
  s.recharge(10000)
  s.recharge(1000)
  const general = s.definePackage(10000, 0, { commissionMode: 'general' })
  s.purchase(general.id, 10000, 0)
  const water = s.definePackage(1000)
  const purchase = s.purchase(water.id, 1000, 0)
  const entry = s.snapshot.commissionEntries.find(item => item.sourceId === purchase.id)!
  assert.equal(entry.rechargeOffset, 120)
  assert.equal(entry.commission, 80)
  s.step(() => updateBrowserPackage(s.snapshot, water.id, { ...water, commissionRate: 0.16 }))
  s.snapshot.employeeCompensations[0]!.baseCommissionRate = 0.15
  normalizeBusinessSnapshot(s.snapshot)
  assert.equal(s.salary('A').generalCommission, 1120)
  assert.equal(s.salary('B').independentCommission, 80)
})

test('功效卡16%，赠送基础水光单独记次数且无额外销售或手工提成；次数和期限同时限制', context => {
  const s = setup(context)
  s.recharge(0)
  const gift = s.definePackage(198)
  const main = s.definePackage(2580, 0.16, {
    giftPackageId: gift.id,
    giftUses: 2,
    totalUses: 6,
    validityDays: 180
  })
  const purchase = s.purchase(main.id, 0, 2580)
  const gifted = s.snapshot.packagePurchases.find(item => item.parentPurchaseId === purchase.id)!
  assert.equal(s.salary('B').independentCommission, 412.8)
  assert.equal(gifted.price, 0)
  assert.equal(gifted.remainingUses, 2)
  assert.equal(gifted.expiresAt, purchase.expiresAt)
  s.step(() =>
    consumeBrowserPackage(s.snapshot, {
      purchaseId: gifted.id,
      employee: 'A',
      duration: 60,
      note: ''
    })
  )
  assert.equal(s.salary('A').commission, 0)
  assert.equal(s.snapshot.services[0]!.commission, 0)
  assert.equal(buildHandworkDetails(s.snapshot, s.salary('A').employeeId, '2026-11').length, 0)
  assert.equal(
    isPackagePurchaseAvailable(purchase, new Date(Date.parse(purchase.expiresAt!) + 1)),
    false
  )
  assert.throws(
    () =>
      refundBrowserPackage(
        s.snapshot,
        { purchaseId: gifted.id, singleOriginalPrice: 198, note: '' },
        '店长'
      ),
    /赠送权益/
  )
})

test('跨月部分退款按原补差提成同比例冲回；退回本金保留原充值来源，再次售卡正确抵扣', context => {
  const s = setup(context)
  s.recharge(1000)
  const water = s.definePackage(1980)
  const purchase = s.purchase(water.id, 1000, 980)
  s.step(() =>
    consumeBrowserPackage(s.snapshot, {
      purchaseId: purchase.id,
      employee: 'B',
      duration: 60,
      note: ''
    })
  )
  s.context.mock.timers.setTime(Date.parse('2026-12-01T01:00:00Z'))
  s.step(() =>
    refundBrowserPackage(
      s.snapshot,
      { purchaseId: purchase.id, singleOriginalPrice: 198, note: '' },
      '店长'
    )
  )
  assert.equal(purchase.refundAmount, 1782)
  assert.equal(purchase.refundBalanceAmount, 900)
  assert.equal(s.salary('B').independentCommission, 296)
  assert.equal(s.salary('B', '2026-12').independentCommission, -266.4)
  const second = s.definePackage(900)
  const next = s.purchase(second.id, 900, 0)
  assert.equal(s.snapshot.commissionEntries.find(item => item.sourceId === next.id)!.commission, 90)
})

test('单次项目赠送余额不计提，多员工共享一次销售提成；跨月撤销冲回并恢复本金来源', context => {
  const s = setup(context)
  s.recharge(100, 'A', 98)
  s.step(() =>
    createBrowserProject(s.snapshot, {
      name: '基础单次',
      duration: 60,
      price: 198,
      productId: null,
      consumptionQuantity: 0,
      commissionMode: 'independent',
      commissionRate: 0.2
    })
  )
  const id = crypto.randomUUID()
  s.step(() =>
    addBrowserService(s.snapshot, {
      requestId: id,
      memberId: s.snapshot.members[0]!.id,
      guestName: '',
      employees: ['A', 'B'],
      projectId: s.snapshot.projects[0]!.id,
      externalPaymentMethod: ''
    })
  )
  const entry = s.snapshot.commissionEntries.find(item => item.sourceId === id)!
  assert.equal(entry.commission, 10)
  assert.deepEqual(
    entry.employees.map(item => item.commission),
    [5, 5]
  )
  assert.equal(s.salary('B').normalServiceCount, 0)
  s.context.mock.timers.setTime(Date.parse('2026-12-01T01:00:00Z'))
  s.step(() => cancelBrowserService(s.snapshot, id))
  assert.equal(s.salary('B', '2026-12').independentCommission, -5)
  assert.equal(s.snapshot.members[0]!.principalBalance, 100)
  assert.equal(s.snapshot.members[0]!.giftBalance, 98)
})

test('六档水光价格全部直接付款时按20%或16%计提，重复修改规则不追溯', context => {
  const s = setup(context)
  s.recharge(0)
  for (const [price, rate, expected] of [
    [198, 0.2, 39.6],
    [990, 0.2, 198],
    [1980, 0.2, 396],
    [498, 0.16, 79.68],
    [2580, 0.16, 412.8],
    [4980, 0.16, 796.8]
  ]) {
    const card = s.definePackage(price!, rate!)
    const purchase = s.purchase(card.id, 0, price!)
    assert.equal(purchase.commission, expected)
  }
})

test('退款恢复原充值批次的FIFO顺序，避免把后充值的其他员工本金优先抵扣', context => {
  const s = setup(context)
  s.recharge(1000)
  const first = s.definePackage(500)
  const original = s.purchase(first.id, 500, 0)
  s.snapshot.employeeCompensations.find(
    item => item.employeeId === s.snapshot.employees.find(item => item.name === 'B')!.id
  )!.baseCommissionRate = 0.12
  s.recharge(1000, 'B')
  s.step(() =>
    refundBrowserPackage(
      s.snapshot,
      { purchaseId: original.id, singleOriginalPrice: 198, note: '' },
      '店长'
    )
  )
  const next = s.definePackage(990)
  const purchase = s.purchase(next.id, 990, 0)
  const entry = s.snapshot.commissionEntries.find(item => item.sourceId === purchase.id)!
  assert.equal(entry.rechargeOffset, 99)
  assert.equal(entry.commission, 99)
})

test('旧普通消费流水优先消耗赠送余额，不误扣充值本金来源', context => {
  const s = setup(context)
  s.recharge(1000, 'A', 200)
  s.step(() => {
    const member = s.snapshot.members[0]!
    member.giftBalance = 50
    member.balance = 1050
    s.snapshot.transactions.unshift({
      id: 'old-spending',
      memberId: member.id,
      memberName: member.name,
      type: 'consume',
      amount: 150,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: 2,
      balanceAfter: 1050,
      paymentMethod: '会员余额',
      item: '历史护理',
      employee: 'A',
      createdAt: new Date().toISOString(),
      note: '',
      status: 'active',
      sourceType: 'legacy'
    })
  })
  const card = s.definePackage(990)
  const purchase = s.purchase(card.id, 990, 0)
  assert.equal(purchase.commission, 99)
})
