import type { ServiceRecord } from '../types'

export function serviceEmployeeNames(
  service: Pick<ServiceRecord, 'employee' | 'employees'>
): string[] {
  const names = (service.employees?.length ? service.employees : [service.employee])
    .map(name => name.trim())
    .filter(Boolean)
  return [...new Set(names)]
}

export function serviceHasEmployee(
  service: Pick<ServiceRecord, 'employee' | 'employees'>,
  employee: string
) {
  return serviceEmployeeNames(service).includes(employee)
}

export function serviceEmployeeLabel(service: Pick<ServiceRecord, 'employee' | 'employees'>) {
  return serviceEmployeeNames(service).join('、')
}
