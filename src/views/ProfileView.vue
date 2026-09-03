<script setup lang="ts">
import { computed } from 'vue'
import { BadgeCheck, KeyRound, ShieldCheck, UserRound } from 'lucide-vue-next'
import { authStore } from '../auth'
import { usePasswordChange } from '../composables/usePasswordChange'

const roleLabel = computed(() => (authStore.user?.role === 'manager' ? '店长账号' : '员工账号'))
const { formRef, form, rules, saving, submit } = usePasswordChange('登录密码修改成功')
</script>

<template>
  <div class="page profile-page">
    <section class="profile-layout">
      <article class="panel account-overview-card">
        <div class="account-large-avatar">{{ authStore.user?.displayName.slice(0, 1) }}</div>
        <h2>{{ authStore.user?.displayName }}</h2>
        <el-tag round :type="authStore.user?.role === 'manager' ? 'danger' : 'primary'">
          <span class="flex-horizontal-center gap-5">
            <BadgeCheck :size="14" />
            {{ roleLabel }}
          </span>
        </el-tag>
        <dl>
          <div>
            <dt>登录账号</dt>
            <dd>{{ authStore.user?.username }}</dd>
          </div>
          <div>
            <dt>账号状态</dt>
            <dd class="active-text">正常使用</dd>
          </div>
          <div>
            <dt>数据权限</dt>
            <dd>{{ authStore.user?.role === 'manager' ? '全部门店数据' : '员工日常操作' }}</dd>
          </div>
        </dl>
      </article>

      <article class="panel password-card">
        <header class="profile-card-head">
          <span><KeyRound :size="22" /></span>
          <div>
            <h3>修改登录密码</h3>
            <p>定期修改密码可以提高账号安全性</p>
          </div>
        </header>
        <el-alert
          title="密码不会以明文形式保存，店长也无法查看原密码。"
          type="success"
          :closable="false"
          show-icon
        />
        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          scroll-to-error
          label-position="top"
          class="password-form"
        >
          <el-form-item label="当前密码" prop="currentPassword">
            <el-input
              v-model="form.currentPassword"
              type="password"
              show-password
              autocomplete="current-password"
            />
          </el-form-item>
          <el-form-item label="新密码" prop="newPassword">
            <el-input
              v-model="form.newPassword"
              type="password"
              show-password
              autocomplete="new-password"
              placeholder="至少 6 位"
            />
          </el-form-item>
          <el-form-item label="确认新密码" prop="confirmPassword">
            <el-input
              v-model="form.confirmPassword"
              type="password"
              show-password
              autocomplete="new-password"
            />
          </el-form-item>
          <el-button type="primary" :loading="saving" @click="submit">
            <ShieldCheck :size="16" />
            保存新密码
          </el-button>
        </el-form>
      </article>
    </section>
    <section class="panel permission-card">
      <UserRound :size="20" />
      <div>
        <h3>当前权限说明</h3>
        <p v-if="authStore.user?.role === 'manager'">
          店长可以管理会员、员工、账号密码、营收报表、数据备份和系统设置。
        </p>
        <p v-else>员工可以查询会员、登记充值消费和服务、查看本人服务记录及修改本人密码。</p>
      </div>
    </section>
  </div>
</template>
