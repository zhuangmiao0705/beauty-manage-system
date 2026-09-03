import { REPORT_RANGES, type ReportRange } from '../config/options'
import type { Employee, ServiceRecord, TransactionRecord } from '../types'
import { dateFromDaysAgo } from '../utils'

export interface AmountTrendRecord {
  createdAt: string
  amount: number
}

export type TrendAggregation = 'sum' | 'average'

export function recordsInRange<T extends { createdAt: string }>(
  records: readonly T[],
  range: ReportRange
) {
  const cutoff = dateFromDaysAgo(REPORT_RANGES[range].days)
  return records.filter(item => new Date(item.createdAt) >= cutoff)
}

export function buildAmountTrend(
  records: readonly AmountTrendRecord[],
  range: ReportRange,
  aggregation: TrendAggregation = 'sum'
) {
  const { days, points: count } = REPORT_RANGES[range]
  const rangeStart = dateFromDaysAgo(days)
  const points = Array.from({ length: count }, (_, index) => {
    const start = new Date(rangeStart)
    const end = new Date(rangeStart)
    if (range === 'day') {
      const slotHours = 24 / count
      start.setHours(index * slotHours, 0, 0, 0)
      end.setHours((index + 1) * slotHours, 0, 0, -1)
    } else {
      start.setDate(start.getDate() + Math.floor((index * days) / count))
      end.setDate(end.getDate() + Math.floor(((index + 1) * days) / count))
      end.setMilliseconds(-1)
    }
    const bucket = records.filter(item => {
      const occurredAt = new Date(item.createdAt)
      return occurredAt >= start && occurredAt <= end
    })
    const total = bucket.reduce((sum, item) => sum + item.amount, 0)
    const amount = aggregation === 'average' && bucket.length ? total / bucket.length : total
    return {
      label:
        range === 'day'
          ? `${String(start.getHours()).padStart(2, '0')}:00`
          : range === 'year'
            ? `${end.getMonth() + 1}月`
            : `${end.getMonth() + 1}/${end.getDate()}`,
      amount
    }
  })
  return points
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
      const records = services.filter(item => item.employee === employee.name)
      return {
        ...employee,
        count: records.length,
        revenue: records.reduce((sum, item) => sum + item.amount, 0)
      }
    })
    .sort((a, b) => b.revenue - a.revenue)
}
