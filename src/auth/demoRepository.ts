import { reactive } from 'vue'
import { APP_CONFIG } from '../config/app'
import type { AccountInput, AccountRecord, AuthResponse, AuthUser } from '../types'

const accounts = reactive<AccountRecord[]>([
  {
    id: 'demo-manager',
    username: APP_CONFIG.defaultManager.username,
    displayName: '木子店长',
    role: 'manager',
    employeeId: null,
    status: 'active',
    mustChangePassword: false,
    createdAt: new Date().toISOString()
  },
  {
    id: 'demo-employee',
    username: 'employee',
    displayName: '员工账号',
    role: 'employee',
    employeeId: null,
    status: 'active',
    mustChangePassword: false,
    createdAt: new Date().toISOString()
  }
])

const passwords = new Map<string, string>([
  ['demo-manager', APP_CONFIG.defaultManager.password],
  ['demo-employee', APP_CONFIG.defaultManager.password]
])

export function loginDemo(username: string, password: string): AuthResponse {
  const account = accounts.find(item => item.username === username)
  if (!account || passwords.get(account.id) !== password) throw new Error('账号或密码错误')
  if (account.status !== 'active') throw new Error('该账号已停用')
  return { token: `demo-${crypto.randomUUID()}`, user: { ...account } }
}

export function changeDemoPassword(user: AuthUser, currentPassword: string, newPassword: string) {
  if (passwords.get(user.id) !== currentPassword) throw new Error('当前密码错误')
  passwords.set(user.id, newPassword)
  const account = accounts.find(item => item.id === user.id)
  if (account) account.mustChangePassword = false
}

export const listDemoAccounts = () => accounts

export function createDemoAccount(input: AccountInput) {
  if (accounts.some(item => item.username === input.username)) throw new Error('登录账号已存在')
  const account: AccountRecord = {
    id: crypto.randomUUID(),
    ...input,
    status: 'active',
    mustChangePassword: false,
    createdAt: new Date().toISOString()
  }
  accounts.push(account)
  passwords.set(account.id, input.password)
  return accounts
}

export function resetDemoPassword(accountId: string, newPassword: string) {
  const account = accounts.find(item => item.id === accountId)
  if (!account) throw new Error('账号不存在')
  passwords.set(accountId, newPassword)
  account.mustChangePassword = false
  return accounts
}

export function setDemoAccountStatus(accountId: string, status: AccountRecord['status']) {
  const account = accounts.find(item => item.id === accountId)
  if (!account) throw new Error('账号不存在')
  account.status = status
  return accounts
}
