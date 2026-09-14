<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { CircleDollarSign } from 'lucide-vue-next'
import { authStore } from '../auth'
import { getMainNavigation, getUtilityNavigation } from '../config/navigation'

defineEmits<{ navigate: [] }>()
const route = useRoute()
const groups = computed(() => getMainNavigation(authStore.user?.role))
const activeGroup = computed(
  () => groups.value.find(group => group.items.some(item => item.to === route.path))?.label
)
const utilityItems = computed(() => getUtilityNavigation(authStore.user?.role))
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">JM</div>
      <div>
        <strong>聚尚木子</strong>
        <span>BEAUTY STUDIO</span>
      </div>
    </div>
    <div class="store-card">
      <span class="store-icon"><CircleDollarSign :size="18" /></span>
      <div>
        <b>聚尚木子旗舰店</b>
        <small>
          <i />
          正常营业中
        </small>
      </div>
    </div>
    <el-menu
      class="sidebar-menu"
      :default-active="route.path"
      :default-openeds="activeGroup ? [activeGroup] : []"
      unique-opened
      router
      @select="$emit('navigate')"
    >
      <el-sub-menu v-for="group in groups" :key="group.label" :index="group.label">
        <template #title>
          <component :is="group.icon" :size="19" />
          <span>{{ group.label }}</span>
        </template>
        <el-menu-item v-for="item in group.items" :key="item.to" :index="item.to">
          <component :is="item.icon" :size="17" />
          <span>{{ item.label }}</span>
        </el-menu-item>
      </el-sub-menu>
    </el-menu>
    <div class="sidebar-spacer" />
    <nav class="bottom-nav">
      <router-link
        v-for="item in utilityItems"
        :key="item.to"
        :to="item.to"
        @click="$emit('navigate')"
      >
        <component :is="item.icon" :size="19" />
        <span>{{ item.label }}</span>
      </router-link>
    </nav>
    <router-link to="/profile" class="profile-card" @click="$emit('navigate')">
      <div class="avatar small">{{ authStore.user?.displayName.slice(0, 1) }}</div>
      <div>
        <b>{{ authStore.user?.displayName }}</b>
        <span>{{ authStore.user?.role === 'manager' ? '店长账号' : '员工账号' }}</span>
      </div>
      <span class="more">•••</span>
    </router-link>
  </aside>
</template>
