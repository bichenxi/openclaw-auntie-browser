<script setup lang="ts">
import { useInstallerStore } from '@/stores/installer'
import { useTabsStore } from '@/stores/tabs'
import { useOnboardStore } from '@/stores/onboard'
import { startInstall, cancelInstall } from '@/api/installer'
import { checkOpenclawAlive } from '@/api/openclaw'
import { restartOpenclawGateway } from '@/api/gateway'
import { useAutoSetup } from '@/composables/useAutoSetup'

const installerStore = useInstallerStore()
const tabsStore = useTabsStore()
const onboardStore = useOnboardStore()
const { autoSetup } = useAutoSetup()

const isWindows = /windows/i.test(navigator.userAgent)
const logContainer = ref<HTMLElement | null>(null)
const checking = ref(false)
const notAliveHint = ref(false)
let pollTimer: ReturnType<typeof setInterval> | null = null

// 一键启动状态（isOnboarded 场景）
const launching = ref(false)
const launchStep = ref<'starting' | 'configuring' | ''>('')
const launchError = ref('')
const launchStepLabel = computed(() => {
  if (launchStep.value === 'starting') return '启动中...'
  if (launchStep.value === 'configuring') return '配置中...'
  return '处理中...'
})

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
      await autoSetup()
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
    installerStore.completeOnboard()
    await autoSetup()
    tabsStore.switchToSpecialView('openclaw')
  } else {
    notAliveHint.value = true
  }
}

async function launchAndSetup() {
  launching.value = true
  launchStep.value = 'starting'
  launchError.value = ''
  try {
    await restartOpenclawGateway().catch(() => {})
    // 轮询直到 gateway 上线，最多等 30 秒
    await new Promise<void>((resolve, reject) => {
      let count = 0
      const timer = setInterval(async () => {
        count++
        const alive = await checkOpenclawAlive().catch(() => false)
        if (alive) {
          clearInterval(timer)
          resolve()
        } else if (count >= 30) {
          clearInterval(timer)
          reject(new Error('启动超时，请确认 OpenClaw 已正确安装'))
        }
      }, 1000)
    })
    launchStep.value = 'configuring'
    await autoSetup()
    tabsStore.switchToSpecialView('openclaw')
  } catch (e: any) {
    launchError.value = e?.message ?? String(e)
    launching.value = false
    launchStep.value = ''
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

    <!-- ── 已安装但未运行（非安装流程内的 needOnboard 状态） ── -->
    <template v-if="installerStore.isInstalled && !installerStore.installing && !installerStore.needOnboard">

      <!-- ── 已完成 onboard：一键启动 ── -->
      <template v-if="installerStore.isOnboarded">
        <div class="flex flex-col items-center gap-3 mb-6">
          <img src="/logo.png" class="w-14 h-14 rounded-[12px] object-cover shadow-lg" alt="logo" />
          <h1 class="text-[22px] font-bold text-[#2d1f6e] m-0">OpenClaw 未运行</h1>
          <p class="text-[13px] text-[#7b6aa8] text-center max-w-[380px] leading-relaxed m-0">
            已检测到完整配置，一键启动即可直接进入对话。
          </p>
        </div>

        <!-- 启动步骤反馈 -->
        <div v-if="launching" class="w-full max-w-[400px] flex flex-col gap-2 mb-5">
          <div
            class="flex items-center gap-2.5 px-4 py-2.5 rounded-lg bg-white border text-[12px] transition"
            :class="launchStep === 'starting' ? 'border-secondary/30 text-secondary' : 'border-[#e8e2f4] text-[#15803d]'"
          >
            <span v-if="launchStep === 'starting'" class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin shrink-0" />
            <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="shrink-0"><polyline points="20 6 9 17 4 12"/></svg>
            启动 Gateway 服务
          </div>
          <div
            class="flex items-center gap-2.5 px-4 py-2.5 rounded-lg bg-white border text-[12px] transition"
            :class="launchStep === 'configuring' ? 'border-secondary/30 text-secondary' : 'border-[#e8e2f4] text-[#c4bdd8]'"
          >
            <span v-if="launchStep === 'configuring'" class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin shrink-0" />
            <span v-else class="w-3 h-3 rounded-full border-2 border-current shrink-0" />
            检测配置并同步认证信息
          </div>
        </div>

        <!-- 错误提示 -->
        <div v-if="launchError" class="w-full max-w-[400px] px-4 py-3 mb-4 rounded-xl bg-red-50 border border-red-200 text-[12px] text-red-600">
          {{ launchError }}
        </div>

        <!-- 主按钮 -->
        <button class="btn mb-5 flex items-center gap-2" :disabled="launching" @click="launchAndSetup">
          <span v-if="launching" class="w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin" />
          {{ launching ? launchStepLabel : '一键启动' }}
        </button>

        <!-- 次级：重新配置向导 -->
        <div class="flex items-center gap-3 w-full max-w-[440px] mb-4">
          <div class="flex-1 h-px bg-[#e8e2f4]" />
          <span class="text-[11px] text-[#c4bdd8] shrink-0">或重新运行配置向导</span>
          <div class="flex-1 h-px bg-[#e8e2f4]" />
        </div>
        <div class="w-full max-w-[440px] flex gap-3">
          <button
            type="button"
            class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-white rounded-xl border border-[#e8e2f4] text-[12px] text-[#8a80a7] cursor-pointer transition hover:text-secondary hover:border-secondary/30 hover:bg-secondary/5"
            @click="onboardStore.openWizard()"
          >
            可视化配置
          </button>
          <button
            v-if="!isWindows"
            type="button"
            class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-white rounded-xl border border-[#e8e2f4] text-[12px] text-[#8a80a7] cursor-pointer transition hover:text-secondary hover:border-secondary/30 hover:bg-secondary/5"
            @click="onboardStore.open()"
          >
            内嵌终端
          </button>
        </div>
      </template>

      <!-- ── 未 onboard：需要初始化向导 ── -->
      <template v-else>
        <div class="flex flex-col items-center gap-3 mb-6">
          <img src="/logo.png" class="w-14 h-14 rounded-[12px] object-cover shadow-lg" alt="logo" />
          <h1 class="text-[22px] font-bold text-[#2d1f6e] m-0">OpenClaw 需要初始化</h1>
          <p class="text-[13px] text-[#7b6aa8] text-center max-w-[380px] leading-relaxed m-0">
            选择一种方式启动配置向导，完成后将自动启动网关。
          </p>
        </div>

        <!-- 配置向导选择区 -->
        <div class="w-full max-w-[440px] flex gap-3 mb-5">
          <!-- 可视化配置 -->
          <button
            type="button"
            class="group flex-1 flex flex-col items-center gap-3 p-5 bg-white rounded-2xl border-2 border-secondary/20 cursor-pointer transition-all hover:border-secondary/50 hover:shadow-[0_4px_20px_rgba(95,71,206,0.12)] active:scale-[0.98]"
            @click="onboardStore.openWizard()"
          >
            <div class="w-12 h-12 rounded-[14px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow-[0_4px_14px_rgba(95,71,206,0.3)]">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                <line x1="3" y1="9" x2="21" y2="9" />
                <line x1="9" y1="21" x2="9" y2="9" />
              </svg>
            </div>
            <div class="text-center">
              <div class="text-[13px] font-bold text-[#2d1f6e] mb-0.5">可视化配置</div>
              <span class="inline-block text-[10px] font-semibold text-white bg-secondary px-2 py-0.5 rounded-full mb-1.5">推荐</span>
              <div class="text-[11px] text-[#9b8ec4] leading-[1.5]">表单步骤引导<br />支持全平台</div>
            </div>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#c4bdd8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mt-auto transition group-hover:stroke-secondary group-hover:translate-x-0.5">
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </button>

          <!-- 内嵌终端（仅 macOS / Linux） -->
          <button
            v-if="!isWindows"
            type="button"
            class="group flex-1 flex flex-col items-center gap-3 p-5 bg-white rounded-2xl border-2 border-[#e8e2f4] cursor-pointer transition-all hover:border-[#c4bdd8] hover:shadow-[0_4px_20px_rgba(26,16,48,0.08)] active:scale-[0.98]"
            @click="onboardStore.open()"
          >
            <div class="w-12 h-12 rounded-[14px] bg-[#1a1030] flex-center shadow-[0_4px_14px_rgba(26,16,48,0.25)]">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#a78bfa" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="4 17 10 11 4 5" />
                <line x1="12" y1="19" x2="20" y2="19" />
              </svg>
            </div>
            <div class="text-center">
              <div class="text-[13px] font-bold text-[#2d1f6e] mb-2">内嵌终端</div>
              <div class="text-[11px] text-[#9b8ec4] leading-[1.5]">直接与 TUI 交互<br />macOS / Linux</div>
            </div>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#c4bdd8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mt-auto transition group-hover:stroke-[#6b5f8a] group-hover:translate-x-0.5">
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </button>
        </div>

        <!-- 分隔线 -->
        <div class="flex items-center gap-3 w-full max-w-[440px] mb-4">
          <div class="flex-1 h-px bg-[#e8e2f4]" />
          <span class="text-[11px] text-[#c4bdd8] shrink-0">或复制命令手动执行</span>
          <div class="flex-1 h-px bg-[#e8e2f4]" />
        </div>

        <!-- 命令展示 -->
        <div class="flex items-center gap-2 bg-[#1a1030] rounded-xl px-5 py-3.5 mb-5 w-full max-w-[340px]">
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

        <p v-if="notAliveHint" class="text-[12px] text-red-500 mb-3 m-0">
          仍未检测到 OpenClaw，请确认命令已执行完毕。
        </p>

        <button class="btn" :disabled="checking" @click="handleCheckAlive">
          {{ checking ? '检测中...' : '我已启动，检测连接' }}
        </button>
      </template>
    </template>

    <!-- ── 安装向导 ── -->
    <template v-else>
      <!-- Header -->
      <div class="flex flex-col items-center gap-3 mb-8">
        <img src="/logo.png" class="w-16 h-16 rounded-[14px] object-cover shadow-lg" alt="logo" />
        <h1 class="text-2xl font-bold text-[#2d1f6e]">安装 OpenClaw</h1>
        <p class="text-[13px] text-[#7b6aa8] text-center max-w-[360px] leading-relaxed">
          Oclaw 需要 OpenClaw 本地 gateway 才能工作。<br />
          点击「开始安装」，将自动完成环境配置与初始化。
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

      <!-- 第三步：初始化配置（needOnboard 时展示） -->
      <template v-if="installerStore.needOnboard">
        <div class="w-full max-w-[400px] mb-4">
          <div class="flex items-center gap-2 px-4 py-3 rounded-xl bg-emerald-50 border border-emerald-200 text-[12px] text-emerald-700 mb-4">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <polyline points="20 6 9 17 4 12" />
            </svg>
            <span class="font-medium">OpenClaw 安装完成，请继续完成初始化配置</span>
          </div>

          <!-- 可视化配置 / 内嵌终端 -->
          <div class="flex gap-3 mb-4">
            <button
              type="button"
              class="group flex-1 flex flex-col items-center gap-2.5 p-4 bg-white rounded-2xl border-2 border-secondary/20 cursor-pointer transition-all hover:border-secondary/50 hover:shadow-[0_4px_20px_rgba(95,71,206,0.12)] active:scale-[0.98]"
              @click="onboardStore.openWizard()"
            >
              <div class="w-10 h-10 rounded-[12px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow-[0_4px_14px_rgba(95,71,206,0.3)]">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                  <line x1="3" y1="9" x2="21" y2="9" />
                  <line x1="9" y1="21" x2="9" y2="9" />
                </svg>
              </div>
              <div class="text-center">
                <div class="text-[12px] font-bold text-[#2d1f6e]">可视化配置</div>
                <span class="inline-block text-[9px] font-semibold text-white bg-secondary px-1.5 py-px rounded-full mt-0.5">推荐</span>
              </div>
            </button>
            <button
              v-if="!isWindows"
              type="button"
              class="group flex-1 flex flex-col items-center gap-2.5 p-4 bg-white rounded-2xl border-2 border-[#e8e2f4] cursor-pointer transition-all hover:border-[#c4bdd8] hover:shadow-[0_4px_20px_rgba(26,16,48,0.08)] active:scale-[0.98]"
              @click="onboardStore.open()"
            >
              <div class="w-10 h-10 rounded-[12px] bg-[#1a1030] flex-center shadow-[0_4px_14px_rgba(26,16,48,0.25)]">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#a78bfa" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="4 17 10 11 4 5" />
                  <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
              </div>
              <div class="text-center">
                <div class="text-[12px] font-bold text-[#2d1f6e]">内嵌终端</div>
                <div class="text-[10px] text-[#9b8ec4] mt-0.5">macOS / Linux</div>
              </div>
            </button>
          </div>
        </div>
      </template>

      <!-- 底部操作区 -->
      <div v-if="!installerStore.needOnboard" class="flex gap-3">
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
