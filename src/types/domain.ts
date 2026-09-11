export type EntityStatus = 'active' | 'inactive'
export type TransactionType = 'recharge' | 'consume'
export type ServiceType = '套盒手工' | '普通手工'
export type PackageType = '套盒' | '普通'
export type PackageLimitType = 'count' | 'time'
export type PackagePaymentMethod = '会员余额' | '现金' | '余额现金组合支付'
export type ExternalPaymentMethod = '微信支付' | '支付宝' | '现金' | '银行卡'
export type AppointmentStatus =
  'pending' | 'arrived' | 'in_service' | 'completed' | 'cancelled' | 'no_show'

export interface Member {
  id: string
  name: string
  phone: string
  balance: number
  principalBalance: number
  giftBalance: number
  totalRecharge: number
  totalConsumption: number
  joinDate: string
  lastVisit: string
  status: EntityStatus
}

export interface TransactionRecord {
  id: string
  memberId: string | null
  memberName: string
  type: TransactionType
  amount: number
  giftAmount: number
  commission: number
  commissionRuleVersion: number
  balanceAfter: number
  paymentMethod: string
  item: string
  employee: string
  createdAt: string
  note: string
  status?: 'active' | 'cancelled'
  sourceType?: 'legacy' | 'service' | 'package_purchase'
  sourceId?: string | null
}

export interface ServiceRecord {
  id: string
  memberId: string | null
  memberName: string
  employee: string
  serviceName: string
  serviceType: ServiceType
  duration: number
  amount: number
  commission: number
  commissionRuleVersion: number
  packagePurchaseId: string | null
  projectId?: string | null
  balancePaymentAmount?: number
  externalPaymentAmount?: number
  paymentMethod?: string
  giftDeduction?: number
  principalDeduction?: number
  transactionId?: string | null
  createdAt: string
  status: 'completed' | 'cancelled'
}

export interface ProjectDefinition {
  id: string
  name: string
  duration: number
  price: number
  status: EntityStatus
  createdAt: string
  updatedAt: string
}

export interface Employee {
  id: string
  name: string
  role: string
  status: EntityStatus
  color: string
  createdAt: string
}

export interface EmployeeStatusEvent {
  id: string
  employeeId: string
  status: Employee['status']
  changedAt: string
}

export interface EmployeeCompensation {
  employeeId: string
  baseSalary: number
  baseCommissionRate: number
  performanceTarget: number
  excessCommissionRate: number
  mealAllowancePerDay: number
  attendanceBonus: number
  normalServiceCommission: number
  packageServiceCommission: number
  createdAt: string
}

export interface CommissionConfig {
  id: string
  effectiveMonth: string
  rechargeRate: number
  packagePurchaseRate: number
  packageServiceAmount: number
  normalServiceAmount: number
  baseSalary: number
  createdAt: string
}

export interface AttendanceRecord {
  id: string
  employeeId: string
  month: string
  restDays: number
  updatedAt: string
}

export interface PackageDefinition {
  id: string
  name: string
  price: number
  totalUses: number
  limitType: PackageLimitType
  validityDays: number
  packageType: PackageType
  status: EntityStatus
  createdAt: string
  updatedAt: string
}

export interface PackagePurchase {
  id: string
  memberId: string
  memberName: string
  packageId: string
  packageName: string
  packageType: PackageType
  employee: string
  price: number
  totalUses: number
  remainingUses: number
  limitType: PackageLimitType
  validityDays: number
  activatedAt: string | null
  expiresAt: string | null
  paymentMethod: PackagePaymentMethod
  balancePaymentAmount: number
  cashPaymentAmount: number
  commission: number
  commissionRuleVersion: number
  purchasedAt: string
  lastConsumedAt: string | null
  status: 'active' | 'completed'
  transactionId: string
}

export interface PackageConsumption {
  id: string
  purchaseId: string
  employee: string
  duration: number
  remainingAfter: number
  createdAt: string
  note: string
  serviceId: string
  status: 'active' | 'cancelled'
  cancelledAt: string | null
}

export interface Appointment {
  id: string
  customerType: 'member' | 'guest'
  memberId: string | null
  customerName: string
  customerPhone: string
  employee: string
  serviceType: ServiceType
  serviceName: string
  projectId?: string | null
  packagePurchaseId: string | null
  startsAt: string
  duration: number
  status: AppointmentStatus
  note: string
  createdBy: string
  createdAt: string
  updatedAt: string
  completedServiceId: string | null
}

export interface EmployeeSalaryRow {
  employeeId: string
  name: string
  rechargePerformance: number
  packagePurchasePerformance: number
  totalPerformance: number
  packageServiceCount: number
  normalServiceCount: number
  restDays: number
  activeDays: number
  workDays: number
  mealDays: number
  baseSalary: number
  commission: number
  mealAllowance: number
  attendanceBonus: number
  totalIncome: number
}

export interface AppSnapshot {
  members: Member[]
  transactions: TransactionRecord[]
  services: ServiceRecord[]
  projects: ProjectDefinition[]
  employees: Employee[]
  employeeCompensations: EmployeeCompensation[]
  employeeStatusEvents: EmployeeStatusEvent[]
  commissionConfigs: CommissionConfig[]
  attendanceRecords: AttendanceRecord[]
  packages: PackageDefinition[]
  packagePurchases: PackagePurchase[]
  packageConsumptions: PackageConsumption[]
  appointments: Appointment[]
}

export interface MemberInput {
  name: string
  phone: string
  initialBalance: number
  giftAmount: number
  employee: string
}

export interface TransactionInput {
  memberId: string
  type: TransactionType
  amount: number
  giftAmount: number
  paymentMethod: string
  item: string
  employee: string
  note: string
}

export interface ServiceInput {
  requestId: string
  memberId: string
  employee: string
  guestName: string
  projectId: string
  externalPaymentMethod: ExternalPaymentMethod | ''
}

export interface ProjectDefinitionInput {
  name: string
  duration: number
  price: number
}

export interface EmployeeInput {
  name: string
  role: string
  color: string
  baseSalary: number
  baseCommissionRate: number
  performanceTarget: number
  excessCommissionRate: number
  mealAllowancePerDay: number
  attendanceBonus: number
  normalServiceCommission: number
  packageServiceCommission: number
}

export type CommissionConfigInput = Omit<CommissionConfig, 'id' | 'createdAt'>

export interface AttendanceInput {
  employeeId: string
  month: string
  restDays: number
}

export interface PackageDefinitionInput {
  name: string
  price: number
  totalUses: number
  limitType: PackageLimitType
  validityDays: number
  packageType: PackageType
}

export interface PackagePurchaseInput {
  memberId: string
  packageId: string
  employee: string
  activatedAt: string
  paymentMethod: PackagePaymentMethod
  balancePaymentAmount: number
  cashPaymentAmount: number
}

export interface PackageConsumptionInput {
  purchaseId: string
  employee: string
  duration: number
  note: string
}

export interface AppointmentInput {
  customerType: Appointment['customerType']
  memberId: string
  guestName: string
  guestPhone: string
  employee: string
  serviceType: ServiceType
  projectId: string
  packagePurchaseId: string
  startsAt: string
  duration: number
  note: string
}

export interface AppointmentCompletionInput {
  appointmentId: string
  duration: number
  externalPaymentMethod: ExternalPaymentMethod | ''
}
