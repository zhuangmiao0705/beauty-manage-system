const currencyFormatters = new Map<number, Intl.NumberFormat>()

export function currency(value: number, digits?: number) {
  const cacheKey = digits ?? -1
  let formatter = currencyFormatters.get(cacheKey)
  if (!formatter) {
    formatter = new Intl.NumberFormat('zh-CN', {
      style: 'currency',
      currency: 'CNY',
      minimumFractionDigits: digits ?? 0,
      maximumFractionDigits: digits ?? 2
    })
    currencyFormatters.set(cacheKey, formatter)
  }
  return formatter.format(value)
}

const dateTimeFormatter = new Intl.DateTimeFormat('zh-CN', {
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hour12: false
})

const dateOnlyFormatter = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit'
})

export const dateTime = (value: string) => dateTimeFormatter.format(new Date(value))
export const dateOnly = (value: string) => dateOnlyFormatter.format(new Date(value))

export function fullDateTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '--'
  const pad = (part: number) => String(part).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}
