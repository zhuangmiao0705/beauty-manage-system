import type { RouteRecordRaw } from 'vue-router'

export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('../views/DashboardView.vue'),
    meta: { title: '经营概览', subtitle: '掌握今日门店经营情况' }
  },
  {
    path: '/members',
    component: () => import('../views/MembersView.vue'),
    meta: { title: '会员管理', subtitle: '会员档案、余额与消费流水' }
  },
  {
    path: '/services',
    component: () => import('../views/ServicesView.vue'),
    meta: { title: '服务记录', subtitle: '记录员工每一次服务与业绩' }
  },
  {
    path: '/projects',
    component: () => import('../views/ProjectsView.vue'),
    meta: { title: '项目管理', subtitle: '配置店内服务项目、时长与价格' }
  },
  {
    path: '/appointments',
    component: () => import('../views/AppointmentsView.vue'),
    meta: { title: '预约管理', subtitle: '安排顾客预约与到店服务流程' }
  },
  {
    path: '/packages',
    component: () => import('../views/PackagesView.vue'),
    meta: { title: '套餐记录', subtitle: '套餐购买、消耗与流水管理' }
  },
  {
    path: '/package-management',
    component: () => import('../views/PackageManagementView.vue'),
    meta: { title: '套餐管理', subtitle: '配置套餐种类、价格、次数与类型', managerOnly: true }
  },
  {
    path: '/revenue',
    component: () => import('../views/RevenueView.vue'),
    meta: { title: '营收统计', subtitle: '多维查看门店与员工业绩', managerOnly: true }
  },
  {
    path: '/staff',
    component: () => import('../views/StaffView.vue'),
    meta: { title: '员工与账号', subtitle: '员工档案、登录账号与权限管理', managerOnly: true }
  },
  {
    path: '/payroll',
    component: () => import('../views/PayrollView.vue'),
    meta: { title: '薪酬管理', subtitle: '员工工资、考勤与提成配置', managerOnly: true }
  },
  {
    path: '/settings',
    component: () => import('../views/SettingsView.vue'),
    meta: { title: '系统设置', subtitle: '数据安全、备份与系统信息', managerOnly: true }
  },
  {
    path: '/profile',
    component: () => import('../views/ProfileView.vue'),
    meta: { title: '个人账号', subtitle: '查看账号信息与修改登录密码' }
  }
]
