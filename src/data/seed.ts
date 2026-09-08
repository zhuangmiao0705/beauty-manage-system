import { COMMISSION_RULE_VERSION, DEFAULT_EMPLOYEE_COMPENSATION } from '../config/options'
import type { AppSnapshot } from '../types'

const day = (offset: number, hour = 10, minute = 0) => {
  const date = new Date()
  date.setDate(date.getDate() + offset)
  date.setHours(hour, minute, 0, 0)
  return date.toISOString()
}

const currentMonth = new Date().toISOString().slice(0, 7)
const seededEmployeeDate = day(-720)

export const seedData: AppSnapshot = {
  projects: [
    {
      id: 'project-1',
      name: '水光补水护理',
      duration: 60,
      price: 398,
      status: 'active',
      createdAt: seededEmployeeDate,
      updatedAt: seededEmployeeDate
    },
    {
      id: 'project-2',
      name: '肩颈舒缓护理',
      duration: 60,
      price: 680,
      status: 'active',
      createdAt: seededEmployeeDate,
      updatedAt: seededEmployeeDate
    },
    {
      id: 'project-3',
      name: '胶原紧致护理',
      duration: 90,
      price: 880,
      status: 'active',
      createdAt: seededEmployeeDate,
      updatedAt: seededEmployeeDate
    }
  ],
  employees: [
    {
      id: 'e1',
      name: '林晓雅',
      role: '高级美容师',
      status: 'active',
      color: '#d86c83',
      createdAt: seededEmployeeDate
    },
    {
      id: 'e2',
      name: '周可欣',
      role: '美容顾问',
      status: 'active',
      color: '#aa7dce',
      createdAt: seededEmployeeDate
    },
    {
      id: 'e3',
      name: '陈思思',
      role: '美容师',
      status: 'active',
      color: '#e7a95f',
      createdAt: seededEmployeeDate
    },
    {
      id: 'e4',
      name: '王小曼',
      role: '美容师',
      status: 'active',
      color: '#65a99b',
      createdAt: seededEmployeeDate
    }
  ],
  employeeCompensations: ['e1', 'e2', 'e3', 'e4'].map(employeeId => ({
    employeeId,
    ...DEFAULT_EMPLOYEE_COMPENSATION,
    createdAt: seededEmployeeDate
  })),
  members: [
    {
      id: 'm1',
      name: '赵女士',
      phone: '138****6621',
      balance: 12860,
      principalBalance: 12860,
      giftBalance: 0,
      totalRecharge: 30000,
      totalConsumption: 17140,
      joinDate: day(-520),
      lastVisit: day(0, 11, 20),
      status: 'active'
    },
    {
      id: 'm2',
      name: '李女士',
      phone: '186****1956',
      balance: 5680,
      principalBalance: 5680,
      giftBalance: 0,
      totalRecharge: 15000,
      totalConsumption: 9320,
      joinDate: day(-340),
      lastVisit: day(-1, 15, 30),
      status: 'active'
    },
    {
      id: 'm3',
      name: '孙女士',
      phone: '139****8073',
      balance: 2360,
      principalBalance: 2360,
      giftBalance: 0,
      totalRecharge: 8000,
      totalConsumption: 5640,
      joinDate: day(-180),
      lastVisit: day(-3, 13, 10),
      status: 'active'
    },
    {
      id: 'm4',
      name: '吴女士',
      phone: '177****2240',
      balance: 860,
      principalBalance: 860,
      giftBalance: 0,
      totalRecharge: 3000,
      totalConsumption: 2140,
      joinDate: day(-76),
      lastVisit: day(-6, 17, 0),
      status: 'active'
    },
    {
      id: 'm5',
      name: '郑女士',
      phone: '135****4188',
      balance: 7350,
      principalBalance: 7350,
      giftBalance: 0,
      totalRecharge: 20000,
      totalConsumption: 12650,
      joinDate: day(-410),
      lastVisit: day(-8, 10, 20),
      status: 'active'
    }
  ],
  transactions: [
    {
      id: 't1',
      memberId: 'm1',
      memberName: '赵女士',
      type: 'consume',
      amount: 1280,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 12860,
      paymentMethod: '会员余额',
      item: '海洋焕肤护理',
      employee: '林晓雅',
      createdAt: day(0, 11, 20),
      note: ''
    },
    {
      id: 't2',
      memberId: 'm2',
      memberName: '李女士',
      type: 'consume',
      amount: 680,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 5680,
      paymentMethod: '会员余额',
      item: '肩颈舒缓护理',
      employee: '周可欣',
      createdAt: day(0, 10, 5),
      note: ''
    },
    {
      id: 't3',
      memberId: 'm3',
      memberName: '孙女士',
      type: 'recharge',
      amount: 3000,
      giftAmount: 0,
      commission: 300,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 2360,
      paymentMethod: '微信支付',
      item: '会员充值',
      employee: '陈思思',
      createdAt: day(-1, 16, 40),
      note: '充值赠送面膜1盒'
    },
    {
      id: 't4',
      memberId: 'm4',
      memberName: '吴女士',
      type: 'consume',
      amount: 398,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 860,
      paymentMethod: '会员余额',
      item: '水光补水护理',
      employee: '王小曼',
      createdAt: day(-2, 14, 15),
      note: ''
    },
    {
      id: 't5',
      memberId: 'm5',
      memberName: '郑女士',
      type: 'consume',
      amount: 880,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 7350,
      paymentMethod: '会员余额',
      item: '胶原紧致护理',
      employee: '林晓雅',
      createdAt: day(-3, 15, 25),
      note: ''
    },
    {
      id: 't6',
      memberId: 'm2',
      memberName: '李女士',
      type: 'recharge',
      amount: 5000,
      giftAmount: 0,
      commission: 500,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 6360,
      paymentMethod: '支付宝',
      item: '会员充值',
      employee: '周可欣',
      createdAt: day(-5, 12, 0),
      note: ''
    },
    {
      id: 't7',
      memberId: 'm1',
      memberName: '赵女士',
      type: 'consume',
      amount: 1680,
      giftAmount: 0,
      commission: 0,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      balanceAfter: 14140,
      paymentMethod: '会员余额',
      item: '抗衰修护疗程',
      employee: '林晓雅',
      createdAt: day(-6, 10, 30),
      note: ''
    }
  ],
  services: [
    {
      id: 's1',
      memberId: 'm1',
      memberName: '赵女士',
      employee: '林晓雅',
      serviceName: '海洋焕肤护理',
      serviceType: '套盒手工',
      duration: 90,
      amount: 1280,
      commission: 10,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: null,
      createdAt: day(0, 11, 20),
      status: 'completed'
    },
    {
      id: 's2',
      memberId: 'm2',
      memberName: '李女士',
      employee: '周可欣',
      serviceName: '肩颈舒缓护理',
      serviceType: '普通手工',
      duration: 60,
      amount: 680,
      commission: 5,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: null,
      createdAt: day(0, 10, 5),
      status: 'completed'
    },
    {
      id: 's3',
      memberId: 'm4',
      memberName: '吴女士',
      employee: '王小曼',
      serviceName: '水光补水护理',
      serviceType: '普通手工',
      duration: 50,
      amount: 398,
      commission: 5,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: null,
      createdAt: day(-2, 14, 15),
      status: 'completed'
    },
    {
      id: 's4',
      memberId: 'm5',
      memberName: '郑女士',
      employee: '林晓雅',
      serviceName: '胶原紧致护理',
      serviceType: '套盒手工',
      duration: 80,
      amount: 880,
      commission: 10,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: null,
      createdAt: day(-3, 15, 25),
      status: 'completed'
    },
    {
      id: 's5',
      memberId: 'm1',
      memberName: '赵女士',
      employee: '林晓雅',
      serviceName: '抗衰修护疗程',
      serviceType: '套盒手工',
      duration: 120,
      amount: 1680,
      commission: 10,
      commissionRuleVersion: COMMISSION_RULE_VERSION,
      packagePurchaseId: null,
      createdAt: day(-6, 10, 30),
      status: 'completed'
    }
  ],
  employeeStatusEvents: [
    ...['e1', 'e2', 'e3', 'e4'].map(employeeId => ({
      id: `event-${employeeId}`,
      employeeId,
      status: 'active' as const,
      changedAt: seededEmployeeDate
    }))
  ],
  commissionConfigs: [
    {
      id: 'commission-default',
      effectiveMonth: '1970-01',
      rechargeRate: 0.1,
      packagePurchaseRate: 0.1,
      packageServiceAmount: 10,
      normalServiceAmount: 5,
      baseSalary: 1800,
      createdAt: seededEmployeeDate
    }
  ],
  attendanceRecords: ['e1', 'e2', 'e3', 'e4'].map(employeeId => ({
    id: `attendance-${employeeId}-${currentMonth}`,
    employeeId,
    month: currentMonth,
    restDays: 0,
    updatedAt: new Date().toISOString()
  })),
  packages: [
    {
      id: 'p1',
      name: '水光焕肤套盒',
      price: 3980,
      totalUses: 10,
      limitType: 'count',
      validityDays: 0,
      packageType: '套盒',
      status: 'active',
      createdAt: day(-60),
      updatedAt: day(-60)
    },
    {
      id: 'p2',
      name: '肩颈舒缓套餐',
      price: 5980,
      totalUses: 12,
      limitType: 'count',
      validityDays: 0,
      packageType: '普通',
      status: 'active',
      createdAt: day(-45),
      updatedAt: day(-45)
    }
  ],
  packagePurchases: [],
  packageConsumptions: [],
  appointments: []
}
