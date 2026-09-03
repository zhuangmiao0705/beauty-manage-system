import {
  BarChart3,
  BadgeDollarSign,
  Boxes,
  CalendarDays,
  ContactRound,
  LayoutDashboard,
  ListTree,
  PackageOpen,
  Settings,
  Sparkles,
  UserCog,
  UsersRound
} from 'lucide-vue-next'
import type { AccountRole } from '../types'

const mainItems = [
  { to: '/', label: '经营概览', icon: LayoutDashboard },
  { to: '/revenue', label: '营收统计', icon: BarChart3, managerOnly: true },
  { to: '/appointments', label: '预约管理', icon: CalendarDays },
  { to: '/services', label: '服务记录', icon: Sparkles },
  { to: '/members', label: '会员管理', icon: UsersRound },
  { to: '/packages', label: '套餐记录', icon: PackageOpen },
  { to: '/package-management', label: '套餐管理', icon: Boxes, managerOnly: true },
  { to: '/projects', label: '项目管理', icon: ListTree },
  { to: '/staff', label: '员工与账号', icon: ContactRound, managerOnly: true },
  { to: '/payroll', label: '薪酬管理', icon: BadgeDollarSign, managerOnly: true }
]

const utilityItems = [
  { to: '/settings', label: '系统设置', icon: Settings, managerOnly: true },
  { to: '/profile', label: '个人账号', icon: UserCog }
]

const visibleTo = (role: AccountRole | undefined) => (item: { managerOnly?: boolean }) =>
  !item.managerOnly || role === 'manager'

export const getMainNavigation = (role?: AccountRole) => mainItems.filter(visibleTo(role))
export const getUtilityNavigation = (role?: AccountRole) => utilityItems.filter(visibleTo(role))
