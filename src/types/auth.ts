import type { EntityStatus } from './domain'

export type AccountRole = 'manager' | 'employee'

export interface AuthUser {
  id: string
  username: string
  displayName: string
  role: AccountRole
  employeeId: string | null
  status: EntityStatus
  mustChangePassword: boolean
}

export interface AuthResponse {
  token: string
  user: AuthUser
}

export interface AccountRecord extends AuthUser {
  createdAt: string
}

export interface AccountInput {
  username: string
  displayName: string
  role: AccountRole
  employeeId: string | null
  password: string
}
