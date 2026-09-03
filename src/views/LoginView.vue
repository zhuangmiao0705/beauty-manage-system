<script setup lang="ts">
import { reactive, ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { LockKeyhole, ShieldCheck, Sparkles, UserRound } from 'lucide-vue-next'
import { login } from '../auth'
import { APP_CONFIG, STORAGE_KEYS } from '../config/app'
import { errorMessage, notify } from '../utils/feedback'
import { requiredTextRule, validateForm } from '../utils/validation'

const loading = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({
  username: localStorage.getItem(STORAGE_KEYS.lastUsername) ?? APP_CONFIG.defaultManager.username,
  password: '',
  rememberUsername: true
})
const rules: FormRules<typeof form> = {
  username: [requiredTextRule('请输入登录账号')],
  password: [{ required: true, message: '请输入登录密码', trigger: ['blur', 'change'] }]
}

async function submit() {
  if (!(await validateForm(formRef.value))) return
  loading.value = true
  try {
    await login(form.username.trim(), form.password)
    if (form.rememberUsername) localStorage.setItem(STORAGE_KEYS.lastUsername, form.username.trim())
    else localStorage.removeItem(STORAGE_KEYS.lastUsername)
  } catch (reason) {
    notify(errorMessage(reason, '登录失败'), 'error')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="login-page">
    <section class="login-brand-panel">
      <div class="login-brand">
        <span>JM</span>
        <div>
          <strong>聚尚木子</strong>
          <small>BEAUTY STUDIO</small>
        </div>
      </div>
      <div class="login-slogan">
        <span>
          <Sparkles :size="18" />
          本地离线管理
        </span>
        <h1>
          让每一次服务
          <br />
          都被认真记录
        </h1>
        <p>会员、员工、服务与营收数据安全保存在本机。</p>
      </div>
      <div class="login-safe">
        <ShieldCheck :size="18" />
        SQLite 本地加密级密码保护
      </div>
    </section>

    <section class="login-form-panel">
      <div class="login-form-card">
        <div class="login-heading">
          <span>欢迎回来</span>
          <h2>登录门店管理系统</h2>
          <p>请输入店长或员工账号继续</p>
        </div>
        <el-form
          ref="formRef"
          :model="form"
          :rules="rules"
          scroll-to-error
          label-position="top"
          @submit.prevent="submit"
        >
          <el-form-item label="登录账号" prop="username">
            <el-input
              v-model="form.username"
              size="large"
              clearable
              autocomplete="username"
              placeholder="请输入登录账号"
              @keyup.enter="submit"
            >
              <template #prefix><UserRound :size="17" /></template>
            </el-input>
          </el-form-item>
          <el-form-item label="登录密码" prop="password">
            <el-input
              v-model="form.password"
              size="large"
              type="password"
              show-password
              autocomplete="current-password"
              placeholder="请输入登录密码"
              @keyup.enter="submit"
            >
              <template #prefix><LockKeyhole :size="17" /></template>
            </el-input>
          </el-form-item>
          <div class="login-options">
            <el-checkbox v-model="form.rememberUsername">记住账号</el-checkbox>
            <span>密码由店长重置</span>
          </div>
          <el-button
            type="primary"
            size="large"
            class="login-submit"
            :loading="loading"
            native-type="submit"
          >
            登录系统
          </el-button>
        </el-form>
        <!-- <div class="initial-account">
          <b>首次使用</b>
          <span>店长账号：{{ APP_CONFIG.defaultManager.username }}</span>
          <span>初始密码：{{ APP_CONFIG.defaultManager.password }}</span>
        </div> -->
      </div>
      <p class="login-copyright">聚尚木子门店管理系统 · 数据由门店自行掌控</p>
    </section>
  </main>
</template>
