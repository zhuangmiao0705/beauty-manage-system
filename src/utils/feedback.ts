import { ElMessage } from 'element-plus'
import 'element-plus/es/components/message/style/css'

export function notify(message: string, type: 'success' | 'error' = 'success') {
  ElMessage({ message, type, duration: 2600, showClose: true, grouping: true })
}

export function errorMessage(reason: unknown, fallback: string) {
  return reason instanceof Error ? reason.message : fallback
}
