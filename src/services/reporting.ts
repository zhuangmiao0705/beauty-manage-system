import type { ReportRange } from '../config/options'
import type { Employee, ServiceRecord, TransactionRecord } from '../types'
import { daysInMonth, localDateKey, localMonthKey, serviceHasEmployee } from '../utils'

export interface AmountTrendRecord {
  createdAt: string
  amount: number
}

export type TrendAggregation = 'sum' | 'average'

export function reportRangeBounds(range: ReportRange, now = new Date()) {
  const year = now.getFullYear()
  const month = now.getMonth()
  if (range === 'day') {
    return {
      start: new Date(year, month, now.getDate()),
      end: new Date(year, month, now.getDate() + 1)
    }
  }
  if (range === 'month') {
    return { start: new Date(year, month, 1), end: new Date(year, month + 1, 1) }
  }
  if (range === 'quarter') {
    const quarterStartMonth = Math.floor(month / 3) * 3
    return {
      start: new Date(year, quarterStartMonth, 1),
      end: new Date(year, quarterStartMonth + 3, 1)
    }
  }
  return { start: new Date(year, 0, 1), end: new Date(year + 1, 0, 1) }
}

export function isInReportRange(value: string, range: ReportRange) {
  const occurredAt = new Date(value)
  const { start, end } = reportRangeBounds(range)
  return occurredAt >= start && occurredAt < end
}

export function recordsInRange<T extends { createdAt: string }>(
  records: readonly T[],
  range: ReportRange
) {
  return records.filter(item => isInReportRange(item.createdAt, range))
}

export function recordsInNaturalMonth<T extends { createdAt: string }>(
  records: readonly T[],
  month: string
) {
  return records.filter(item => localMonthKey(item.createdAt) === month)
}

export function recordsInNaturalYear<T extends { createdAt: string }>(
  records: readonly T[],
  year: string
) {
  return records.filter(item => localMonthKey(item.createdAt).startsWith(`${year}-`))
}

function aggregateBucket(
  records: readonly AmountTrendRecord[],
  predicate: (createdAt: string) => boolean,
  aggregation: TrendAggregation
) {
  const bucket = records.filter(item => predicate(item.createdAt))
  const total = bucket.reduce((sum, item) => sum + item.amount, 0)
  return aggregation === 'average' && bucket.length ? total / bucket.length : total
}

export function buildAmountTrend(
  records: readonly AmountTrendRecord[],
  range: ReportRange,
  aggregation: TrendAggregation = 'sum'
) {
  const now = new Date()
  if (range === 'month')
    return buildNaturalMonthAmountTrend(records, localMonthKey(now), aggregation)
  if (range === 'year')
    return buildNaturalYearAmountTrend(records, String(now.getFullYear()), aggregation)

  const { start } = reportRangeBounds(range, now)
  if (range === 'quarter') {
    return Array.from({ length: 3 }, (_, index) => {
      const month = start.getMonth() + index + 1
      const monthKey = `${start.getFullYear()}-${String(month).padStart(2, '0')}`
      return {
        label: `${month}月`,
        amount: aggregateBucket(
          records,
          createdAt => localMonthKey(createdAt) === monthKey,
          aggregation
        )
      }
    })
  }

  const count = 8
  return Array.from({ length: count }, (_, index) => {
    const bucketStart = new Date(start)
    bucketStart.setHours(index * 3, 0, 0, 0)
    const bucketEnd = new Date(start)
    bucketEnd.setHours((index + 1) * 3, 0, 0, 0)
    return {
      label: `${String(bucketStart.getHours()).padStart(2, '0')}:00`,
      amount: aggregateBucket(
        records,
        createdAt => {
          const occurredAt = new Date(createdAt)
          return occurredAt >= bucketStart && occurredAt < bucketEnd
        },
        aggregation
      )
    }
  })
}

export function buildNaturalMonthAmountTrend(
  records: readonly AmountTrendRecord[],
  month: string,
  aggregation: TrendAggregation = 'sum'
) {
  const dayCount = daysInMonth(month)
  const [year, monthNumber] = month.split('-').map(Number)
  if (!year || !monthNumber || monthNumber < 1 || monthNumber > 12 || !dayCount) return []

  return Array.from({ length: dayCount }, (_, index) => {
    const day = index + 1
    const dateKey = `${year}-${String(monthNumber).padStart(2, '0')}-${String(day).padStart(2, '0')}`
    return {
      label: `${monthNumber}/${day}`,
      amount: aggregateBucket(
        records,
        createdAt => localDateKey(createdAt) === dateKey,
        aggregation
      )
    }
  })
}

export function buildNaturalYearAmountTrend(
  records: readonly AmountTrendRecord[],
  year: string,
  aggregation: TrendAggregation = 'sum'
) {
  const yearNumber = Number(year)
  if (!Number.isInteger(yearNumber) || year.length !== 4) return []

  return Array.from({ length: 12 }, (_, index) => {
    const month = index + 1
    const monthKey = `${year}-${String(month).padStart(2, '0')}`
    return {
      label: `${month}月`,
      amount: aggregateBucket(
        records,
        createdAt => localMonthKey(createdAt) === monthKey,
        aggregation
      )
    }
  })
}

export function buildRevenueTrend(transactions: readonly TransactionRecord[], range: ReportRange) {
  return buildAmountTrend(
    transactions
      .filter(item => item.type === 'consume' && item.status !== 'cancelled')
      .map(item => ({ createdAt: item.createdAt, amount: item.amount })),
    range
  )
}

export function buildEmployeeStats(
  employees: readonly Employee[],
  services: readonly ServiceRecord[]
) {
  return employees
    .map(employee => {
      const records = services.filter(item => serviceHasEmployee(item, employee.name))
      return {
        ...employee,
        count: records.length,
        revenue: records.reduce((sum, item) => sum + item.amount, 0)
      }
    })
    .sort((a, b) => b.revenue - a.revenue)
}
