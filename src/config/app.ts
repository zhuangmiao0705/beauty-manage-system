export const APP_CONFIG = {
  name: '聚尚木子门店管理系统',
  storeName: '聚尚木子旗舰店',
  version: '0.2.0',
  defaultManager: { username: 'admin', password: '123456' }
} as const

export const STORAGE_KEYS = {
  salonData: 'jumeimuzi-demo-v1',
  lastUsername: 'salon-last-username'
} as const
