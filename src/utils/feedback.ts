import { ElMessage } from 'element-plus'
import 'element-plus/es/components/message/style/css'

export function notify(message: string, type: 'success' | 'error' = 'success') {
  ElMessage({
    message,
    type,
    duration: type === 'error' ? 5000 : 2600,
    showClose: true,
    grouping: true
  })
}

export function errorMessage(reason: unknown, fallback: string) {
  if (typeof reason === 'string' && reason.trim()) return reason.trim()
  if (reason instanceof Error && reason.message.trim()) return reason.message.trim()
  if (reason && typeof reason === 'object') {
    const error = reason as { message?: unknown; error?: unknown }
    if (typeof error.message === 'string' && error.message.trim()) return error.message.trim()
    if (typeof error.error === 'string' && error.error.trim()) return error.error.trim()
    if (error.error instanceof Error && error.error.message.trim()) {
      return error.error.message.trim()
    }
  }
  return fallback
}
