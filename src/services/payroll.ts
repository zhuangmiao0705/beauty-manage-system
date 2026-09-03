import {
  DEFAULT_ATTENDANCE_REST_DAYS,
  DEFAULT_EMPLOYEE_COMPENSATION,
  STANDARD_MONTHLY_REST_DAYS
} from '../config/options'
import type { AppSnapshot, Employee, EmployeeSalaryRow, ServiceType } from '../types'
import { daysInMonth, isInMonth } from '../utils'

const roundMoney = (value: number) => Math.round(value * 100) / 100

export interface HandworkDetailRow {
  id: string
  memberName: string
  memberPhone: string
  serviceName: string
  serviceType: ServiceType
  duration: number
  commission: number
  createdAt: string
}

export interface PerformanceDetailRow {
  id: string
  memberName: string
  memberPhone: string
  source: '会员充值' | '套餐额外支付'
  businessName: string
  businessAmount: number
  giftAmount: number
  balancePaymentAmount: number
  performanceAmount: number
  paymentMethod: string
  createdAt: string
}

function compensationForEmployee(snapshot: AppSnapshot, employee: Employee) {
  return (
    snapshot.employeeCompensations.find(item => item.employeeId === employee.id) ?? {
      employeeId: employee.id,
      ...DEFAULT_EMPLOYEE_COMPENSATION
    }
  )
}

function memberPhone(snapshot: AppSnapshot, memberId: string | null) {
  if (!memberId) return ''
  return snapshot.members.find(item => item.id === memberId)?.phone ?? ''
}

export function buildHandworkDetails(
  snapshot: AppSnapshot,
  employeeId: string,
  month: string
): HandworkDetailRow[] {
  const employee = snapshot.employees.find(item => item.id === employeeId)
  if (!employee) return []
  const compensation = compensationForEmployee(snapshot, employee)
  return snapshot.services
    .filter(
      item =>
        item.status === 'completed' &&
        item.employee === employee.name &&
        isInMonth(item.createdAt, month)
    )
    .map(item => ({
      id: item.id,
      memberName: item.memberName,
      memberPhone: memberPhone(snapshot, item.memberId),
      serviceName: item.serviceName,
      serviceType: item.serviceType,
      duration: item.duration,
      commission:
        item.serviceType === '套盒手工'
          ? compensation.packageServiceCommission
          : compensation.normalServiceCommission,
      createdAt: item.createdAt
    }))
    .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
}

export function buildPerformanceDetails(
  snapshot: AppSnapshot,
  employeeId: string,
  month: string
): PerformanceDetailRow[] {
  const employee = snapshot.employees.find(item => item.id === employeeId)
  if (!employee) return []
  const rechargeDetails: PerformanceDetailRow[] = snapshot.transactions
    .filter(
      item =>
        item.type === 'recharge' &&
        item.employee === employee.name &&
        isInMonth(item.createdAt, month)
    )
    .map(item => ({
      id: `recharge-${item.id}`,
      memberName: item.memberName,
      memberPhone: memberPhone(snapshot, item.memberId),
      source: '会员充值',
      businessName: item.item || '会员充值',
      businessAmount: item.amount,
      giftAmount: item.giftAmount,
      balancePaymentAmount: 0,
      performanceAmount: item.amount,
      paymentMethod: item.paymentMethod,
      createdAt: item.createdAt
    }))
  const packageDetails: PerformanceDetailRow[] = snapshot.packagePurchases
    .filter(item => item.employee === employee.name && isInMonth(item.purchasedAt, month))
    .map(item => ({
      id: `package-${item.id}`,
      memberName: item.memberName,
      memberPhone: memberPhone(snapshot, item.memberId),
      source: '套餐额外支付',
      businessName: item.packageName,
      businessAmount: item.price,
      giftAmount: 0,
      balancePaymentAmount: item.balancePaymentAmount,
      performanceAmount: item.cashPaymentAmount,
      paymentMethod: item.paymentMethod,
      createdAt: item.purchasedAt
    }))
  return [...rechargeDetails, ...packageDetails].sort((a, b) =>
    b.createdAt.localeCompare(a.createdAt)
  )
}

export function configForMonth(snapshot: AppSnapshot, month: string) {
  return [...snapshot.commissionConfigs]
    .filter(item => item.effectiveMonth <= month)
    .sort((a, b) => b.effectiveMonth.localeCompare(a.effectiveMonth))[0]
}

export function employeeActiveDays(
  snapshot: AppSnapshot,
  employee: Employee,
  month: string,
  dayLimit = daysInMonth(month)
) {
  const [year, monthNumber] = month.split('-').map(Number)
  const count = Math.min(daysInMonth(month), Math.max(0, dayLimit))
  const events = snapshot.employeeStatusEvents
    .filter(item => item.employeeId === employee.id)
    .sort((a, b) => a.changedAt.localeCompare(b.changedAt))
  let activeDays = 0
  for (let day = 1; day <= count; day += 1) {
    const endOfDay = new Date(year, monthNumber - 1, day, 23, 59, 59, 999)
    if (endOfDay < new Date(employee.createdAt)) continue
    const latest = events.filter(item => new Date(item.changedAt) <= endOfDay).at(-1)
    if ((latest?.status ?? 'active') === 'active') activeDays += 1
  }
  return activeDays
}

function elapsedDaysInMonth(month: string) {
  const currentMonth = new Date().toLocaleDateString('sv-SE').slice(0, 7)
  if (month < currentMonth) return daysInMonth(month)
  if (month > currentMonth) return 0
  return new Date().getDate()
}

export function buildSalaryRows(snapshot: AppSnapshot, month: string): EmployeeSalaryRow[] {
  const standardWorkDays = Math.max(1, daysInMonth(month) - STANDARD_MONTHLY_REST_DAYS)
  const elapsedDays = elapsedDaysInMonth(month)
  return snapshot.employees
    .map(employee => {
      const activeDays = employeeActiveDays(snapshot, employee, month)
      const elapsedActiveDays = employeeActiveDays(snapshot, employee, month, elapsedDays)
      const compensation = compensationForEmployee(snapshot, employee)
      const restDays =
        snapshot.attendanceRecords.find(
          item => item.employeeId === employee.id && item.month === month
        )?.restDays ?? DEFAULT_ATTENDANCE_REST_DAYS
      const workDays = Math.max(0, activeDays - restDays)
      const mealDays = Math.max(0, elapsedActiveDays - restDays)
      const recharges = snapshot.transactions.filter(
        item =>
          item.type === 'recharge' &&
          item.employee === employee.name &&
          isInMonth(item.createdAt, month)
      )
      const purchases = snapshot.packagePurchases.filter(
        item => item.employee === employee.name && isInMonth(item.purchasedAt, month)
      )
      const packageServices = snapshot.services.filter(
        item =>
          item.status === 'completed' &&
          item.serviceType === '套盒手工' &&
          item.employee === employee.name &&
          isInMonth(item.createdAt, month)
      )
      const normalServices = snapshot.services.filter(
        item =>
          item.status === 'completed' &&
          item.serviceType === '普通手工' &&
          item.employee === employee.name &&
          isInMonth(item.createdAt, month)
      )
      const baseSalary = roundMoney((compensation.baseSalary / standardWorkDays) * workDays)
      const rechargePerformance = recharges.reduce((sum, item) => sum + item.amount, 0)
      const packagePurchasePerformance = purchases.reduce(
        (sum, item) => sum + item.cashPaymentAmount,
        0
      )
      const totalPerformance = roundMoney(rechargePerformance + packagePurchasePerformance)
      const commission = roundMoney(
        totalPerformance * compensation.baseCommissionRate +
          Math.max(0, totalPerformance - compensation.performanceTarget) *
            compensation.excessCommissionRate +
          packageServices.length * compensation.packageServiceCommission +
          normalServices.length * compensation.normalServiceCommission
      )
      const mealAllowance = roundMoney(mealDays * compensation.mealAllowancePerDay)
      const attendanceBonus =
        mealDays >= standardWorkDays ? roundMoney(compensation.attendanceBonus) : 0
      return {
        employeeId: employee.id,
        name: employee.name,
        rechargePerformance,
        packagePurchasePerformance,
        totalPerformance,
        packageServiceCount: packageServices.length,
        normalServiceCount: normalServices.length,
        restDays,
        activeDays,
        workDays,
        mealDays,
        baseSalary,
        commission,
        mealAllowance,
        attendanceBonus,
        totalIncome: roundMoney(baseSalary + commission + mealAllowance + attendanceBonus)
      }
    })
    .filter(item => item.activeDays > 0)
}
