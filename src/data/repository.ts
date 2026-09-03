import { reactive, readonly } from 'vue'
import { invokeCommand, isTauriRuntime } from '../platform/tauri'
import { authStore } from '../auth/store'
import type {
  AppSnapshot,
  AppointmentCompletionInput,
  AppointmentInput,
  AppointmentStatus,
  AttendanceInput,
  CommissionConfigInput,
  EmployeeInput,
  MemberInput,
  PackageConsumptionInput,
  PackageDefinitionInput,
  PackagePurchaseInput,
  ProjectDefinitionInput,
  ServiceInput,
  TransactionInput
} from '../types'
import {
  completeBrowserAppointment,
  createBrowserAppointment,
  setBrowserAppointmentStatus,
  updateBrowserAppointment
} from './browserAppointmentRepository'
import {
  addBrowserEmployee,
  addBrowserMember,
  addBrowserService,
  addBrowserTransaction,
  cancelBrowserService,
  downloadBrowserBackup,
  loadBrowserSnapshot,
  saveBrowserSnapshot,
  setBrowserEmployeeStatus
} from './browserRepository'
import {
  consumeBrowserPackage,
  createBrowserProject,
  createBrowserPackage,
  purchaseBrowserPackage,
  saveBrowserAttendance,
  saveBrowserCommissionConfig,
  setBrowserProjectStatus,
  setBrowserPackageStatus,
  updateBrowserPackage,
  updateBrowserProject
} from './browserBusinessRepository'

const emptySnapshot = (): AppSnapshot => ({
  members: [],
  transactions: [],
  services: [],
  projects: [],
  employees: [],
  employeeCompensations: [],
  employeeStatusEvents: [],
  commissionConfigs: [],
  attendanceRecords: [],
  packages: [],
  packagePurchases: [],
  packageConsumptions: [],
  appointments: []
})

const state = reactive<AppSnapshot>(emptySnapshot())
let initialized = false
let authToken = ''

function setState(snapshot: AppSnapshot) {
  state.members.splice(0, state.members.length, ...snapshot.members)
  state.transactions.splice(0, state.transactions.length, ...snapshot.transactions)
  state.services.splice(0, state.services.length, ...snapshot.services)
  state.projects.splice(0, state.projects.length, ...snapshot.projects)
  state.employees.splice(0, state.employees.length, ...snapshot.employees)
  state.employeeCompensations.splice(
    0,
    state.employeeCompensations.length,
    ...snapshot.employeeCompensations
  )
  state.employeeStatusEvents.splice(
    0,
    state.employeeStatusEvents.length,
    ...snapshot.employeeStatusEvents
  )
  state.commissionConfigs.splice(0, state.commissionConfigs.length, ...snapshot.commissionConfigs)
  state.attendanceRecords.splice(0, state.attendanceRecords.length, ...snapshot.attendanceRecords)
  state.packages.splice(0, state.packages.length, ...snapshot.packages)
  state.packagePurchases.splice(0, state.packagePurchases.length, ...snapshot.packagePurchases)
  state.packageConsumptions.splice(
    0,
    state.packageConsumptions.length,
    ...snapshot.packageConsumptions
  )
  state.appointments.splice(0, state.appointments.length, ...snapshot.appointments)
}

const desktopCall = <T>(command: string, args?: Record<string, unknown>) =>
  invokeCommand<T>(command, { token: authToken, ...args })

function persistBrowserMutation(mutation: (snapshot: AppSnapshot) => void) {
  mutation(state)
  saveBrowserSnapshot(state)
}

export function setRepositoryToken(token: string) {
  authToken = token
}

export function clearSalonStore() {
  initialized = false
  setState(emptySnapshot())
}

export async function initializeStore(force = false) {
  if (initialized && !force) return
  setState(
    isTauriRuntime() ? await desktopCall<AppSnapshot>('get_snapshot') : loadBrowserSnapshot()
  )
  initialized = true
}

export async function addMember(input: MemberInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_member', { input }))
  else persistBrowserMutation(snapshot => addBrowserMember(snapshot, input))
}

export async function addTransaction(input: TransactionInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_transaction', { input }))
  else persistBrowserMutation(snapshot => addBrowserTransaction(snapshot, input))
}

export async function addService(input: ServiceInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_service', { input }))
  else persistBrowserMutation(snapshot => addBrowserService(snapshot, input))
}

export async function cancelService(serviceId: string) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('cancel_service', { serviceId }))
  else persistBrowserMutation(snapshot => cancelBrowserService(snapshot, serviceId))
}

export async function addProject(input: ProjectDefinitionInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_project', { input }))
  else persistBrowserMutation(snapshot => createBrowserProject(snapshot, input))
}

export async function updateProject(projectId: string, input: ProjectDefinitionInput) {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('update_project', { projectId, input }))
  else persistBrowserMutation(snapshot => updateBrowserProject(snapshot, projectId, input))
}

export async function setProjectStatus(projectId: string, status: 'active' | 'inactive') {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('set_project_status', { projectId, status }))
  else persistBrowserMutation(snapshot => setBrowserProjectStatus(snapshot, projectId, status))
}

export async function addEmployee(input: EmployeeInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_employee', { input }))
  else persistBrowserMutation(snapshot => addBrowserEmployee(snapshot, input))
}

export async function setEmployeeStatus(employeeId: string, status: 'active' | 'inactive') {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('set_employee_status', { employeeId, status }))
  else persistBrowserMutation(snapshot => setBrowserEmployeeStatus(snapshot, employeeId, status))
}

export async function updateCommissionConfig(input: CommissionConfigInput) {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('update_commission_config', { input }))
  else persistBrowserMutation(snapshot => saveBrowserCommissionConfig(snapshot, input))
}

export async function upsertAttendance(input: AttendanceInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('upsert_attendance', { input }))
  else persistBrowserMutation(snapshot => saveBrowserAttendance(snapshot, input))
}

export async function addPackage(input: PackageDefinitionInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_package', { input }))
  else persistBrowserMutation(snapshot => createBrowserPackage(snapshot, input))
}

export async function updatePackage(packageId: string, input: PackageDefinitionInput) {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('update_package', { packageId, input }))
  else persistBrowserMutation(snapshot => updateBrowserPackage(snapshot, packageId, input))
}

export async function setPackageStatus(packageId: string, status: 'active' | 'inactive') {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('set_package_status', { packageId, status }))
  else persistBrowserMutation(snapshot => setBrowserPackageStatus(snapshot, packageId, status))
}

export async function purchasePackage(input: PackagePurchaseInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('purchase_package', { input }))
  else persistBrowserMutation(snapshot => purchaseBrowserPackage(snapshot, input))
}

export async function consumePackage(input: PackageConsumptionInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('consume_package', { input }))
  else persistBrowserMutation(snapshot => consumeBrowserPackage(snapshot, input))
}

export async function createAppointment(input: AppointmentInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('create_appointment', { input }))
  else
    persistBrowserMutation(snapshot =>
      createBrowserAppointment(snapshot, input, authStore.user?.displayName ?? '本地账号')
    )
}

export async function updateAppointment(appointmentId: string, input: AppointmentInput) {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('update_appointment', { appointmentId, input }))
  else persistBrowserMutation(snapshot => updateBrowserAppointment(snapshot, appointmentId, input))
}

export async function setAppointmentStatus(appointmentId: string, status: AppointmentStatus) {
  if (isTauriRuntime())
    setState(await desktopCall<AppSnapshot>('set_appointment_status', { appointmentId, status }))
  else
    persistBrowserMutation(snapshot => setBrowserAppointmentStatus(snapshot, appointmentId, status))
}

export async function completeAppointment(input: AppointmentCompletionInput) {
  if (isTauriRuntime()) setState(await desktopCall<AppSnapshot>('complete_appointment', { input }))
  else persistBrowserMutation(snapshot => completeBrowserAppointment(snapshot, input))
}

export async function createBackup() {
  if (isTauriRuntime()) return desktopCall<string>('create_backup')
  downloadBrowserBackup(state)
  return '已下载到浏览器默认下载目录'
}

export async function restoreLatestBackup() {
  if (!isTauriRuntime()) throw new Error('桌面应用中才支持一键恢复；浏览器演示版请使用导出的文件。')
  setState(await desktopCall<AppSnapshot>('restore_latest_backup'))
  return '最近一次备份已恢复'
}

export const salonStore = readonly(state)
