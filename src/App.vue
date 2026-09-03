<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import { useRoute, useRouter } from 'vue-router'
import { Bell, ChevronDown, LogOut, Menu, Search, UserRound, X } from 'lucide-vue-next'
import AppSidebar from './components/AppSidebar.vue'
import LoginView from './views/LoginView.vue'
import { authStore, logout } from './auth'
import { clearSalonStore, initializeStore, setRepositoryToken } from './data/repository'
import { errorMessage } from './utils/feedback'

const route = useRoute()
const router = useRouter()
const loading = ref(true)
const error = ref('')
const mobileMenu = ref(false)
const title = computed(() => String(route.meta.title ?? '聚尚木子'))
const subtitle = computed(() => String(route.meta.subtitle ?? ''))
const today = new Intl.DateTimeFormat('zh-CN', {
  month: 'long',
  day: 'numeric',
  weekday: 'long'
}).format(new Date())

async function handleLogin() {
  loading.value = true
  error.value = ''
  setRepositoryToken(authStore.token)
  try {
    await initializeStore(true)
    if (authStore.user?.role === 'employee' && route.meta.managerOnly) {
      await router.replace('/services')
    }
  } catch (reason) {
    error.value = errorMessage(reason, '数据加载失败')
  } finally {
    loading.value = false
  }
}

watch(
  () => authStore.user?.id,
  userId => {
    if (userId) void handleLogin()
  },
  { flush: 'sync' }
)

async function handleLogout() {
  await logout()
  clearSalonStore()
  setRepositoryToken('')
  await router.replace('/')
}

async function handleAccountCommand(command: string) {
  if (command === 'profile') await router.push('/profile')
  if (command === 'logout') await handleLogout()
}
</script>

<template>
  <el-config-provider :locale="zhCn">
    <LoginView v-if="!authStore.user" />
    <div v-else class="app-shell">
      <AppSidebar :class="{ open: mobileMenu }" @navigate="mobileMenu = false" />
      <div v-if="mobileMenu" class="mobile-overlay" @click="mobileMenu = false" />

      <main class="main-area">
        <header class="topbar">
          <div class="topbar-title">
            <el-button circle class="menu-button" @click="mobileMenu = !mobileMenu">
              <X v-if="mobileMenu" :size="20" />
              <Menu v-else :size="20" />
            </el-button>
            <div>
              <h1>{{ title }}</h1>
              <p>{{ subtitle }} · {{ today }}</p>
            </div>
          </div>
          <div class="topbar-actions">
            <el-dropdown trigger="click" @command="handleAccountCommand">
              <el-button text class="account-trigger">
                <div class="avatar">{{ authStore.user.displayName.slice(0, 1) }}</div>
                <div class="account-trigger-text">
                  <b>{{ authStore.user.displayName }}</b>
                  <span>{{ authStore.user.role === 'manager' ? '店长' : '员工' }}</span>
                </div>
                <ChevronDown :size="14" />
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="profile">
                    <UserRound :size="15" />
                    个人账号
                  </el-dropdown-item>
                  <el-dropdown-item command="logout" divided>
                    <LogOut :size="15" />
                    退出登录
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </header>

        <div v-if="loading" class="loading-screen">
          <div class="loading-mark">JM</div>
          <p>正在加载门店数据…</p>
        </div>
        <div v-else-if="error" class="error-card">{{ error }}</div>
        <router-view v-else />
      </main>
    </div>
  </el-config-provider>
</template>
