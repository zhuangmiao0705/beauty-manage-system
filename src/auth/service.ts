import { invokeCommand, isTauriRuntime } from '../platform/tauri'
import type { AccountInput, AccountRecord, AuthResponse, AuthUser } from '../types'
import {
  changeDemoPassword,
  createDemoAccount,
  listDemoAccounts,
  loginDemo,
  resetDemoPassword,
  setDemoAccountStatus
} from './demoRepository'
import { authState, clearAuthState } from './store'

export async function login(username: string, password: string) {
  const response = isTauriRuntime()
    ? await invokeCommand<AuthResponse>('login', { username, password })
    : loginDemo(username, password)
  authState.token = response.token
  authState.user = response.user
  return response.user
}

export async function logout() {
  if (isTauriRuntime() && authState.token) {
    try {
      await invokeCommand('logout', { token: authState.token })
    } catch {
      // 本地会话可能已经过期，客户端仍需完成退出。
    }
  }
  clearAuthState()
}

export async function changeOwnPassword(currentPassword: string, newPassword: string) {
  if (!authState.user) throw new Error('请先登录')
  if (newPassword.length < 6) throw new Error('新密码至少需要 6 位')
  if (isTauriRuntime()) {
    authState.user = await invokeCommand<AuthUser>('change_password', {
      token: authState.token,
      currentPassword,
      newPassword
    })
  } else {
    changeDemoPassword(authState.user, currentPassword, newPassword)
    authState.user.mustChangePassword = false
  }
  return authState.user
}

export async function getAccounts() {
  if (isTauriRuntime())
    return invokeCommand<AccountRecord[]>('get_accounts', { token: authState.token })
  return listDemoAccounts()
}

export async function createAccount(input: AccountInput) {
  if (isTauriRuntime())
    return invokeCommand<AccountRecord[]>('create_account', { token: authState.token, input })
  return createDemoAccount(input)
}

export async function resetAccountPassword(accountId: string, newPassword: string) {
  if (isTauriRuntime()) {
    return invokeCommand<AccountRecord[]>('reset_account_password', {
      token: authState.token,
      accountId,
      newPassword
    })
  }
  return resetDemoPassword(accountId, newPassword)
}

export async function setAccountStatus(accountId: string, status: AccountRecord['status']) {
  if (isTauriRuntime()) {
    return invokeCommand<AccountRecord[]>('set_account_status', {
      token: authState.token,
      accountId,
      status
    })
  }
  return setDemoAccountStatus(accountId, status)
}
