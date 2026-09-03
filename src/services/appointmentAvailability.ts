import { APPOINTMENT_BUSINESS_HOURS } from '../config/options'
import type { Appointment, Employee } from '../types'
import { localDateKey } from '../utils'

export interface AvailabilitySlot {
  startsAt: string
  endsAt: string
  startLabel: string
  endLabel: string
  duration: number
}

export interface EmployeeAvailability {
  employeeId: string
  employeeName: string
  color: string
  busyMinutes: number
  freeMinutes: number
  slots: AvailabilitySlot[]
}

type TimeRange = { start: number; end: number }

function dateAtTime(date: string, time: string) {
  return new Date(`${date}T${time}:00`)
}

function roundUpToHalfHour(date: Date) {
  const rounded = new Date(date)
  rounded.setSeconds(0, 0)
  rounded.setMinutes(Math.ceil(rounded.getMinutes() / 30) * 30)
  return rounded
}

function timeLabel(timestamp: number) {
  const date = new Date(timestamp)
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
}

function mergeRanges(ranges: TimeRange[]) {
  return ranges
    .sort((a, b) => a.start - b.start)
    .reduce<TimeRange[]>((merged, current) => {
      const last = merged.at(-1)
      if (!last || current.start > last.end) merged.push({ ...current })
      else last.end = Math.max(last.end, current.end)
      return merged
    }, [])
}

export function buildEmployeeAvailability(
  employees: readonly Employee[],
  appointments: readonly Appointment[],
  date: string,
  minimumDuration: number,
  now = new Date()
): EmployeeAvailability[] {
  if (!date || minimumDuration <= 0) return []
  const businessStart = dateAtTime(date, APPOINTMENT_BUSINESS_HOURS.start).getTime()
  const businessEnd = dateAtTime(date, APPOINTMENT_BUSINESS_HOURS.end).getTime()
  const today = localDateKey(now)
  const freeWindowStart =
    date === today ? Math.max(businessStart, roundUpToHalfHour(now).getTime()) : businessStart
  if (date < today || freeWindowStart >= businessEnd)
    return employees
      .filter(employee => employee.status === 'active')
      .map(employee => ({
        employeeId: employee.id,
        employeeName: employee.name,
        color: employee.color,
        busyMinutes: 0,
        freeMinutes: 0,
        slots: []
      }))

  return employees
    .filter(employee => employee.status === 'active')
    .map(employee => {
      const busyRanges = mergeRanges(
        appointments
          .filter(
            appointment =>
              appointment.employee === employee.name &&
              appointment.status !== 'cancelled' &&
              appointment.status !== 'no_show' &&
              localDateKey(appointment.startsAt) === date
          )
          .map(appointment => {
            const start = new Date(appointment.startsAt).getTime()
            return {
              start: Math.max(start, freeWindowStart),
              end: start + appointment.duration * 60_000
            }
          })
          .filter(range => range.end > freeWindowStart && range.start < businessEnd)
          .map(range => ({ start: range.start, end: Math.min(range.end, businessEnd) }))
      )
      const slots: AvailabilitySlot[] = []
      let cursor = freeWindowStart
      busyRanges.forEach(range => {
        if (range.start > cursor) {
          const duration = Math.floor((range.start - cursor) / 60_000)
          if (duration >= minimumDuration)
            slots.push({
              startsAt: new Date(cursor).toISOString(),
              endsAt: new Date(range.start).toISOString(),
              startLabel: timeLabel(cursor),
              endLabel: timeLabel(range.start),
              duration
            })
        }
        cursor = Math.max(cursor, range.end)
      })
      if (businessEnd > cursor) {
        const duration = Math.floor((businessEnd - cursor) / 60_000)
        if (duration >= minimumDuration)
          slots.push({
            startsAt: new Date(cursor).toISOString(),
            endsAt: new Date(businessEnd).toISOString(),
            startLabel: timeLabel(cursor),
            endLabel: timeLabel(businessEnd),
            duration
          })
      }
      const busyMinutes = busyRanges.reduce(
        (total, range) => total + Math.max(0, (range.end - range.start) / 60_000),
        0
      )
      return {
        employeeId: employee.id,
        employeeName: employee.name,
        color: employee.color,
        busyMinutes: Math.round(busyMinutes),
        freeMinutes: slots.reduce((total, slot) => total + slot.duration, 0),
        slots
      }
    })
}
