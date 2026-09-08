import type {
  AppointmentStatus,
  PackageLimitType,
  PackageType,
  ServiceType,
  TransactionType
} from '../types'

export const RECHARGE_PAYMENT_METHODS = ['微信支付', '支付宝', '现金', '银行卡'] as const
export const PACKAGE_PAYMENT_METHODS = ['会员余额', '现金', '余额现金组合支付'] as const
export const EMPLOYEE_ROLES = ['美容师', '高级美容师', '美容顾问', '前台'] as const
export const SERVICE_TYPES: ServiceType[] = ['套盒手工', '普通手工']
export const SERVICE_TYPE_LABELS: Record<ServiceType, string> = {
  普通手工: '普通消费',
  套盒手工: '套盒消费'
}
export const PACKAGE_TYPES: PackageType[] = ['套盒', '普通']
export const PACKAGE_LIMIT_TYPES: Array<{ value: PackageLimitType; label: string }> = [
  { value: 'count', label: '次数限制' },
  { value: 'time', label: '时间限制' }
]
export const APPOINTMENT_STATUS_OPTIONS: Array<{
  value: AppointmentStatus
  label: string
  type: 'primary' | 'warning' | 'success' | 'info' | 'danger'
}> = [
  { value: 'pending', label: '待到店', type: 'primary' },
  { value: 'arrived', label: '已到店', type: 'warning' },
  { value: 'in_service', label: '服务中', type: 'warning' },
  { value: 'completed', label: '已完成', type: 'success' },
  { value: 'cancelled', label: '已取消', type: 'info' },
  { value: 'no_show', label: '已爽约', type: 'danger' }
]
export const APPOINTMENT_BUSINESS_HOURS = {
  start: '09:00',
  end: '21:00'
} as const
export const DEFAULT_COMMISSION_CONFIG = {
  rechargeRate: 0.1,
  packagePurchaseRate: 0.1,
  packageServiceAmount: 10,
  normalServiceAmount: 5,
  baseSalary: 1800
} as const
export const STANDARD_MONTHLY_REST_DAYS = 4
export const DEFAULT_ATTENDANCE_REST_DAYS = 0
export const COMMISSION_RULE_VERSION = 2
export const PACKAGE_COMMISSION_RULE_VERSION = 3
export const DEFAULT_EMPLOYEE_COMPENSATION = {
  baseSalary: 1800,
  baseCommissionRate: 0.1,
  performanceTarget: 10000,
  excessCommissionRate: 0.02,
  mealAllowancePerDay: 10,
  attendanceBonus: 300,
  normalServiceCommission: 5,
  packageServiceCommission: 10
} as const

export const TRANSACTION_DEFAULTS: Record<
  TransactionType,
  { paymentMethod: string; item: string }
> = {
  recharge: { paymentMethod: '微信支付', item: '会员充值' },
  consume: { paymentMethod: '会员余额', item: '' }
}

export const REPORT_RANGES = {
  day: { label: '今日', days: 1, points: 8 },
  month: { label: '近30天', days: 30, points: 10 },
  quarter: { label: '近一季度', days: 90, points: 12 },
  year: { label: '近一年', days: 365, points: 12 }
} as const

export type ReportRange = keyof typeof REPORT_RANGES
