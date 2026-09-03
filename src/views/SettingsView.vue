<script setup lang="ts">
import { ref } from 'vue'
import { ElMessageBox } from 'element-plus'
import 'element-plus/es/components/message-box/style/css'
import {
  ArchiveRestore,
  CheckCircle2,
  CloudDownload,
  Database,
  Download,
  HardDrive,
  Info,
  RefreshCw,
  ShieldCheck
} from 'lucide-vue-next'
import { APP_CONFIG } from '../config/app'
import { createBackup, restoreLatestBackup } from '../data/repository'
import { isTauriRuntime } from '../platform/tauri'
import { errorMessage, notify } from '../utils/feedback'

const busy = ref('')
async function backup() {
  busy.value = 'backup'
  try {
    const path = await createBackup()
    notify(`备份成功：${path}`)
  } catch (reason) {
    notify(errorMessage(reason, '备份失败'), 'error')
  } finally {
    busy.value = ''
  }
}
async function restore() {
  try {
    await ElMessageBox.confirm('恢复前会自动保存当前数据库。确定继续吗？', '恢复最近备份', {
      type: 'warning',
      confirmButtonText: '确认恢复',
      cancelButtonText: '取消'
    })
  } catch {
    return
  }
  busy.value = 'restore'
  try {
    await restoreLatestBackup()
    notify('数据恢复成功')
  } catch (reason) {
    notify(errorMessage(reason, '恢复失败'), 'error')
  } finally {
    busy.value = ''
  }
}
async function checkUpdate() {
  busy.value = 'update'
  try {
    if (!isTauriRuntime()) {
      notify('浏览器演示模式：当前已是最新版本')
      return
    }
    notify('自动更新服务尚未启用，请使用安装包覆盖更新')
    return
    /* 启用正式更新服务器后恢复以下代码：
    const { check } = await import('@tauri-apps/plugin-updater')
    const update = await check()
    if (!update?.available) notify('当前已是最新版本')
    else if (confirm(`发现新版本 ${update.version}，是否立即更新？`)) {
      await update.downloadAndInstall()
      const { relaunch } = await import('@tauri-apps/plugin-process')
      await relaunch()
    }
    */
  } catch (reason) {
    notify(errorMessage(reason, '检查更新失败'), 'error')
  } finally {
    busy.value = ''
  }
}
</script>

<template>
  <div class="page settings-page">
    <section class="settings-hero">
      <span><ShieldCheck :size="28" /></span>
      <div>
        <h2>数据安全中心</h2>
        <p>会员余额与资金流水保存在本机 SQLite 数据库中，程序更新不会覆盖数据。</p>
      </div>
      <div class="safe-state">
        <CheckCircle2 :size="18" />
        数据保护已启用
      </div>
    </section>
    <section class="settings-grid">
      <article class="panel settings-card">
        <header>
          <span class="settings-icon rose"><Database :size="22" /></span>
          <div>
            <h3>本地数据库</h3>
            <p>正式应用使用 SQLite WAL 模式</p>
          </div>
        </header>
        <dl>
          <div>
            <dt>数据库状态</dt>
            <dd>
              <i />
              运行正常
            </dd>
          </div>
          <div>
            <dt>存储位置</dt>
            <dd>系统应用数据目录</dd>
          </div>
          <div>
            <dt>同步模式</dt>
            <dd>FULL · 安全优先</dd>
          </div>
        </dl>
      </article>
      <article class="panel settings-card">
        <header>
          <span class="settings-icon gold"><HardDrive :size="22" /></span>
          <div>
            <h3>自动备份</h3>
            <p>创建独立、可恢复的数据副本</p>
          </div>
        </header>
        <dl>
          <div>
            <dt>备份策略</dt>
            <dd>每日首次启动</dd>
          </div>
          <div>
            <dt>保留数量</dt>
            <dd>最近 30 份</dd>
          </div>
          <div>
            <dt>最近备份</dt>
            <dd>等待首次备份</dd>
          </div>
        </dl>
        <el-button
          type="primary"
          class="block"
          :loading="busy === 'backup'"
          :disabled="!!busy && busy !== 'backup'"
          @click="backup"
        >
          <Download :size="17" />
          立即创建备份
        </el-button>
      </article>
      <article class="panel settings-card">
        <header>
          <span class="settings-icon green"><ArchiveRestore :size="22" /></span>
          <div>
            <h3>数据恢复</h3>
            <p>从最近一次正常备份恢复</p>
          </div>
        </header>
        <div class="setting-note">恢复前会自动保存当前数据库。建议只在数据异常或误操作时使用。</div>
        <el-button
          class="block"
          :loading="busy === 'restore'"
          :disabled="!!busy && busy !== 'restore'"
          @click="restore"
        >
          <RefreshCw :size="17" />
          恢复最近备份
        </el-button>
      </article>
      <article class="panel settings-card">
        <header>
          <span class="settings-icon violet"><CloudDownload :size="22" /></span>
          <div>
            <h3>应用更新</h3>
            <p>安全下载并验证新版安装包</p>
          </div>
        </header>
        <dl>
          <div>
            <dt>当前版本</dt>
            <dd>{{ APP_CONFIG.version }}</dd>
          </div>
          <div>
            <dt>更新通道</dt>
            <dd>稳定版</dd>
          </div>
          <div>
            <dt>数据库迁移</dt>
            <dd>更新前自动备份</dd>
          </div>
        </dl>
        <el-button
          class="block"
          :loading="busy === 'update'"
          :disabled="!!busy && busy !== 'update'"
          @click="checkUpdate"
        >
          <RefreshCw :size="17" />
          检查应用更新
        </el-button>
      </article>
    </section>
    <section class="panel about-card">
      <Info :size="20" />
      <div>
        <h3>聚尚木子门店管理系统</h3>
        <p>Vue 3 + Tauri 2 + SQLite · 本地离线运行 · 数据由门店自行掌控</p>
      </div>
      <span>Version {{ APP_CONFIG.version }}</span>
    </section>
  </div>
</template>
