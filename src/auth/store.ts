import { reactive, readonly } from 'vue'
import type { AuthUser } from '../types'

export const authState = reactive<{ token: string; user: AuthUser | null }>({
  token: '',
  user: null
})

export const authStore = readonly(authState)

export function clearAuthState() {
  authState.token = ''
  authState.user = null
}
