<script setup lang="ts">
import { computed } from 'vue'
import { CircleDollarSign } from 'lucide-vue-next'
import { authStore } from '../auth'
import { getMainNavigation, getUtilityNavigation } from '../config/navigation'

defineEmits<{ navigate: [] }>()
const items = computed(() => getMainNavigation(authStore.user?.role))
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
    <nav>
      <p class="nav-caption">门店管理</p>
      <router-link v-for="item in items" :key="item.to" :to="item.to" @click="$emit('navigate')">
        <component :is="item.icon" :size="19" />
        <span>{{ item.label }}</span>
      </router-link>
    </nav>
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
