<script setup lang="ts">
import { useInstallerStore } from '@/stores/installer'
import { useTabsStore } from '@/stores/tabs'
import { startInstall, cancelInstall } from '@/api/installer'
import { checkOpenclawAlive } from '@/api/openclaw'

const installerStore = useInstallerStore()
const tabsStore = useTabsStore()

const logContainer = ref<HTMLElement | null>(null)
const checking = ref(false)
const notAliveHint = ref(false)  // 用户点"检测"但仍未连接时提示
let pollTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  installerStore.startListeners()
})

onUnmounted(() => {
  installerStore.stopListeners()
  if (pollTimer) clearInterval(pollTimer)
})

// installer:done → 轮询 gateway 直到上线，再跳转
watch(
  () => installerStore.done,
  (val) => {
    if (!val) return
    startPolling()
  },
)

// 日志区自动滚动
watch(
  () => installerStore.logs.length,
  async () => {
    await nextTick()
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  },
)

function startPolling() {
  if (pollTimer) clearInterval(pollTimer)
  pollTimer = setInterval(async () => {
    const alive = await checkOpenclawAlive().catch(() => false)
    if (alive) {
      clearInterval(pollTimer!)
      pollTimer = null
      tabsStore.switchToSpecialView('openclaw')
    }
  }, 1000)
}

async function handleCheckAlive() {
  checking.value = true
  notAliveHint.value = false
  const alive = await checkOpenclawAlive().catch(() => false)
  checking.value = false
  if (alive) {
    tabsStore.switchToSpecialView('openclaw')
  } else {
    notAliveHint.value = true
  }
}

async function handleStart() {
  installerStore.resetSteps()
  installerStore.installing = true
  await startInstall().catch((e: Error) => {
    installerStore.installing = false
    installerStore.error = e?.message ?? String(e)
  })
}

async function handleCancel() {
  await cancelInstall().catch(() => {})
  installerStore.installing = false
}

function copyCommand() {
  navigator.clipboard.writeText('openclaw onboard').catch(() => {})
}
</script>

<template>
  <div class="flex flex-col items-center justify-center h-full bg-[linear-gradient(180deg,#f8f6ff_0%,#f3eeff_100%)] p-6">

    <!-- ── 已安装但未运行 ── -->
    <template v-if="installerStore.isInstalled && !installerStore.installing">
      <div class="flex flex-col items-center gap-3 mb-8">
        <img src="/logo.jpg" class="w-16 h-16 rounded-[14px] object-cover shadow-lg" alt="logo" />
        <h1 class="text-2xl font-bold text-[#2d1f6e]">OpenClaw 未运行</h1>
        <p class="text-[13px] text-[#7b6aa8] text-center max-w-[340px] leading-relaxed">
          检测到 OpenClaw 已安装，但 gateway 尚未启动。<br />
          请打开终端，执行以下命令：
        </p>
      </div>

      <!-- 命令展示 -->
      <div class="flex items-center gap-2 bg-[#1a1030] rounded-xl px-5 py-3.5 mb-6 w-full max-w-[340px]">
        <code class="flex-1 text-green-400 font-mono text-[14px] select-all">openclaw onboard</code>
        <button
          class="flex-shrink-0 text-[#9b8ec4] hover:text-white transition-colors cursor-pointer bg-transparent border-none p-0"
          title="复制命令"
          @click="copyCommand"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        </button>
      </div>

      <!-- 未检测到提示 -->
      <p v-if="notAliveHint" class="text-[12px] text-red-500 mb-3">
        仍未检测到 OpenClaw，请确认命令已执行完毕。
      </p>

      <button class="btn" :disabled="checking" @click="handleCheckAlive">
        {{ checking ? '检测中...' : '我已启动，检测连接' }}
      </button>
    </template>

    <!-- ── 安装向导 ── -->
    <template v-else>
      <!-- Header -->
      <div class="flex flex-col items-center gap-3 mb-8">
        <img src="/logo.jpg" class="w-16 h-16 rounded-[14px] object-cover shadow-lg" alt="logo" />
        <h1 class="text-2xl font-bold text-[#2d1f6e]">安装 OpenClaw</h1>
        <p class="text-[13px] text-[#7b6aa8] text-center max-w-[360px] leading-relaxed">
          Claw Browser 需要 OpenClaw 本地 gateway 才能工作。<br />
          点击「开始安装」，应用将自动下载 Node.js 并安装 OpenClaw。
        </p>
      </div>

      <!-- 步骤列表 -->
      <div class="w-full max-w-[400px] bg-white rounded-2xl shadow-sm border border-[#e8e2f4] overflow-hidden mb-4">
        <div
          v-for="(step, idx) in installerStore.steps"
          :key="step.id"
          class="flex items-center gap-3 px-5 py-3.5"
          :class="idx < installerStore.steps.length - 1 ? 'border-b border-[#f0ecfa]' : ''"
        >
          <div class="w-6 h-6 flex-shrink-0 flex-center">
            <div v-if="step.status === 'pending'" class="w-5 h-5 rounded-full border-2 border-[#d4cdf4]" />
            <div v-else-if="step.status === 'running'" class="w-5 h-5 rounded-full border-2 border-[#e8e2f4] border-t-[#5f47ce] animate-spin" />
            <svg v-else-if="step.status === 'done'" class="w-5 h-5 text-emerald-500" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd" />
            </svg>
            <svg v-else-if="step.status === 'error'" class="w-5 h-5 text-red-500" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
            </svg>
          </div>
          <span
            class="text-[13px] flex-1"
            :class="{
              'text-[#9b8ec4]': step.status === 'pending',
              'text-[#5f47ce] font-medium': step.status === 'running',
              'text-[#2d1f6e]': step.status === 'done',
              'text-red-500': step.status === 'error',
            }"
          >{{ step.label }}</span>
        </div>
      </div>

      <!-- 终端输出区 -->
      <div
        v-if="installerStore.installing || installerStore.logs.length > 0"
        ref="logContainer"
        class="w-full max-w-[400px] h-[140px] bg-[#1a1030] rounded-xl p-3 overflow-y-auto mb-4 font-mono text-[11px] leading-relaxed"
      >
        <div v-for="(line, i) in installerStore.logs" :key="i" class="text-green-400 whitespace-pre-wrap break-all">
          {{ line }}
        </div>
      </div>

      <!-- 错误提示 -->
      <div
        v-if="installerStore.error"
        class="w-full max-w-[400px] rounded-xl bg-red-50 border border-red-200 px-4 py-3 mb-4 text-[12px] text-red-600"
      >
        安装失败：{{ installerStore.error }}
      </div>

      <!-- 底部操作区 -->
      <div class="flex gap-3">
        <button v-if="!installerStore.installing && !installerStore.done" class="btn" @click="handleStart">
          {{ installerStore.error ? '重试' : '开始安装' }}
        </button>
        <button v-if="installerStore.installing" disabled class="btn opacity-60 cursor-not-allowed">
          安装中...
        </button>
        <button v-if="installerStore.installing" class="btn-plain" @click="handleCancel">
          取消
        </button>
        <button v-if="installerStore.done" class="btn" @click="tabsStore.switchToSpecialView('openclaw')">
          开始使用
        </button>
      </div>
    </template>

  </div>
</template>
