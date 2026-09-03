import { createRouter, createWebHashHistory } from 'vue-router'
import { authStore } from '../auth'
import { routes } from './routes'

export const router = createRouter({ history: createWebHashHistory(), routes })

router.beforeEach(to => {
  if (to.meta.managerOnly && authStore.user?.role !== 'manager') return '/services'
  return true
})
