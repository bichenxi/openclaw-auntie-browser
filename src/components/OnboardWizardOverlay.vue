<script setup lang="ts">
import { useOnboardStore } from '@/stores/onboard'
import {
  startOnboardWizard,
  wizardSendKey,
  killOnboardWizard,
  type WizardPrompt,
} from '@/api/onboard'
import { restartOpenclawGateway } from '@/api/gateway'
import { checkOpenclawAlive } from '@/api/openclaw'
import { useTabsStore } from '@/stores/tabs'
import { listen } from '@tauri-apps/api/event'

const store = useOnboardStore()
const tabsStore = useTabsStore()
const unlistens = ref<Array<() => void>>([])
const starting = ref(false)
let pollTimer: ReturnType<typeof setInterval> | null = null

function startListeners() {
  listen<WizardPrompt>('wizard:prompt', (e) => {
    const prev = store.wizardPrompt
    if (prev && prev.prompt_type !== 'done') {
      store.wizardHistory.push({
        question: prev.question,
        answer: prev.prompt_type === 'confirm'
          ? prev.options[prev.selected] ?? ''
          : prev.prompt_type === 'select'
            ? prev.options[prev.selected] ?? ''
            : '...',
      })
    }
    store.wizardPrompt = e.payload
    store.wizardInputValue = ''
  }).then((fn) => unlistens.value.push(fn))

  listen<{ code: number }>('wizard:exited', (e) => {
    store.wizardRunning = false
    store.wizardExitCode = e.payload.code
    if (e.payload.code === 0) {
      startGateway()
    } else {
      store.wizardError = `进程退出码 ${e.payload.code}`
    }
  }).then((fn) => unlistens.value.push(fn))
}

function stopListeners() {
  unlistens.value.forEach((fn) => fn())
  unlistens.value = []
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
}

watch(
  () => store.wizardVisible,
  (visible) => {
    if (visible) startListeners()
    else stopListeners()
  },
)

onMounted(() => { if (store.wizardVisible) startListeners() })
onUnmounted(stopListeners)

async function handleStart() {
  starting.value = true
  store.wizardError = null
  store.wizardExitCode = null
  store.wizardPrompt = null
  store.wizardHistory = []
  try {
    await startOnboardWizard()
    store.wizardRunning = true
  } catch (e: unknown) {
    store.wizardError = (e as Error)?.message ?? String(e)
  } finally {
    starting.value = false
  }
}

async function answerConfirm(choice: number) {
  const prompt = store.wizardPrompt
  if (!prompt) return
  if (choice === 0 && prompt.selected !== 0) {
    await wizardSendKey('left')
  } else if (choice === 1 && prompt.selected !== 1) {
    await wizardSendKey('right')
  }
  await new Promise((r) => setTimeout(r, 50))
  await wizardSendKey('enter')
}

async function answerSelect(index: number) {
  const prompt = store.wizardPrompt
  if (!prompt) return
  const diff = index - prompt.selected
  if (diff > 0) {
    for (let i = 0; i < diff; i++) await wizardSendKey('down')
  } else if (diff < 0) {
    for (let i = 0; i < -diff; i++) await wizardSendKey('up')
  }
  await new Promise((r) => setTimeout(r, 50))
  await wizardSendKey('enter')
}

async function answerInput() {
  const text = store.wizardInputValue.trim()
  if (!text) return
  await wizardSendKey(`submit:${text}`)
}

async function startGateway() {
  store.wizardStartingGateway = true
  try {
    await restartOpenclawGateway()
  } catch (_) {
    // 即使 restart 失败，也开始轮询（可能是第一次 start）
  }
  pollTimer = setInterval(async () => {
    const alive = await checkOpenclawAlive().catch(() => false)
    if (alive) {
      if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
      store.wizardGatewayDone = true
      store.wizardStartingGateway = false
    }
  }, 1500)
  // 30 秒超时
  setTimeout(() => {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
      if (!store.wizardGatewayDone) {
        store.wizardStartingGateway = false
        store.wizardError = '网关启动超时，请手动执行 openclaw gateway start'
      }
    }
  }, 30000)
}

function handleClose() {
  if (store.wizardRunning) killOnboardWizard().catch(() => {})
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
  store.closeWizard()
}

function goToChat() {
  store.closeWizard()
  tabsStore.switchToSpecialView('openclaw')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="store.wizardVisible"
        class="fixed inset-0 z-[9999] flex items-center justify-center"
      >
        <div
          class="absolute inset-0 bg-black/40 backdrop-blur-sm"
          @click="!store.wizardRunning && !store.wizardStartingGateway && handleClose()"
        />

        <div class="relative w-full max-w-[520px] mx-4 bg-white rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-[#f0ecfa] shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-[9px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                </svg>
              </div>
              <div>
                <div class="text-[15px] font-bold text-[#1f1f2e]">OpenClaw 初始化</div>
                <div class="text-[11px] text-[#9b8ec4]">
                  {{ store.wizardGatewayDone ? '初始化完成' : store.wizardStartingGateway ? '正在启动网关…' : store.wizardRunning ? '配置向导' : '点击开始' }}
                </div>
              </div>
            </div>
            <button
              v-if="!store.wizardRunning && !store.wizardStartingGateway"
              class="w-7 h-7 flex-center rounded-lg text-[#9b8ec4] hover:bg-[#f5f3ff] hover:text-secondary transition cursor-pointer bg-transparent border-none"
              @click="handleClose()"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <!-- Body -->
          <div class="flex-1 min-h-0 p-6 flex flex-col overflow-y-auto gap-4">
            <!-- 未启动：开始按钮 -->
            <template v-if="!store.wizardRunning && !store.wizardPrompt && store.wizardExitCode === null && !store.wizardStartingGateway && !store.wizardGatewayDone">
              <div class="flex flex-col items-center gap-4 py-8">
                <img src="/logo.jpg" class="w-14 h-14 rounded-[12px] object-cover shadow-lg" alt="logo" />
                <p class="text-[13px] text-[#6b5f8a] text-center max-w-[360px] leading-relaxed">
                  将运行 <code class="bg-[#f0ecfa] px-1.5 py-px rounded text-[11px]">openclaw onboard</code> 进行初始化配置，完成后自动启动网关。
                </p>
                <button
                  class="px-6 py-2.5 text-[14px] font-medium text-white rounded-[10px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_10px_rgba(95,71,206,0.22)] hover:shadow-[0_4px_16px_rgba(95,71,206,0.32)] disabled:opacity-50 disabled:cursor-not-allowed"
                  :disabled="starting"
                  @click="handleStart"
                >
                  {{ starting ? '启动中…' : '开始初始化' }}
                </button>
              </div>
            </template>

            <!-- 历史记录 -->
            <div
              v-for="(item, i) in store.wizardHistory"
              :key="i"
              class="flex items-start gap-2 px-4 py-2.5 rounded-xl bg-[#faf9ff] border border-[#f0ecfa]"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#5f47ce" stroke-width="2.5" class="mt-px shrink-0">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <div>
                <div class="text-[12px] text-[#6b5f8a]">{{ item.question }}</div>
                <div class="text-[13px] font-medium text-secondary">{{ item.answer }}</div>
              </div>
            </div>

            <!-- 当前 prompt 卡片 -->
            <template v-if="store.wizardPrompt && store.wizardRunning">
              <!-- Confirm -->
              <div v-if="store.wizardPrompt.prompt_type === 'confirm'" class="flex flex-col gap-3">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ store.wizardPrompt.question }}</p>
                <div class="flex gap-3">
                  <button
                    v-for="(opt, i) in store.wizardPrompt.options"
                    :key="i"
                    type="button"
                    class="flex-1 py-2.5 text-[13px] font-medium rounded-xl border cursor-pointer transition"
                    :class="i === 0
                      ? 'border-secondary bg-secondary/8 text-secondary hover:bg-secondary/15'
                      : 'border-[#e8e2f4] text-[#6b5f8a] hover:border-secondary/30 hover:bg-secondary/5'"
                    @click="answerConfirm(i)"
                  >
                    {{ opt }}
                  </button>
                </div>
              </div>

              <!-- Select -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'select'" class="flex flex-col gap-2">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ store.wizardPrompt.question }}</p>
                <button
                  v-for="(opt, i) in store.wizardPrompt.options"
                  :key="i"
                  type="button"
                  class="flex items-center gap-3 px-4 py-3 rounded-xl border text-left transition cursor-pointer"
                  :class="store.wizardPrompt.selected === i
                    ? 'border-secondary bg-secondary/8 text-secondary'
                    : 'border-[#e8e2f4] bg-white text-[#4a4568] hover:border-secondary/30 hover:bg-secondary/5'"
                  @click="answerSelect(i)"
                >
                  <span class="w-4 h-4 rounded-full border-2 flex-center shrink-0" :class="store.wizardPrompt.selected === i ? 'border-secondary bg-secondary' : 'border-[#c4bdd8]'">
                    <span v-if="store.wizardPrompt.selected === i" class="w-1.5 h-1.5 rounded-full bg-white" />
                  </span>
                  <span class="text-[13px] font-medium">{{ opt }}</span>
                </button>
              </div>

              <!-- Input / Password -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'input' || store.wizardPrompt.prompt_type === 'password'" class="flex flex-col gap-3">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ store.wizardPrompt.question }}</p>
                <input
                  v-model="store.wizardInputValue"
                  :type="store.wizardPrompt.prompt_type === 'password' ? 'password' : 'text'"
                  class="w-full px-4 py-3 text-[13px] border border-[#e8e2f4] rounded-xl outline-none focus:border-secondary focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)]"
                  placeholder="请输入…"
                  autocomplete="off"
                  @keyup.enter="answerInput()"
                />
                <button
                  type="button"
                  class="self-end px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] disabled:opacity-50 disabled:cursor-not-allowed"
                  :disabled="!store.wizardInputValue.trim()"
                  @click="answerInput()"
                >
                  确认
                </button>
              </div>

              <!-- Done (from TUI) -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'done'" class="flex items-center gap-3 px-4 py-4 rounded-xl bg-emerald-50 border border-emerald-200 text-emerald-700">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                <span class="text-[13px] font-medium">{{ store.wizardPrompt.question }}</span>
              </div>

              <!-- Info / 未知类型：显示纯文本 -->
              <div v-else class="px-4 py-3 rounded-xl bg-[#faf9ff] border border-[#f0ecfa] text-[12px] text-[#6b5f8a]">
                {{ store.wizardPrompt.question }}
              </div>
            </template>

            <!-- 正在启动网关 -->
            <div v-if="store.wizardStartingGateway" class="flex items-center gap-3 px-4 py-4 rounded-xl bg-[#faf9ff] border border-[#f0ecfa]">
              <span class="w-4 h-4 border-2 border-secondary border-t-transparent rounded-full animate-spin shrink-0" />
              <span class="text-[13px] text-[#4a4568]">正在启动 OpenClaw 网关，请稍候…</span>
            </div>

            <!-- 网关已启动 -->
            <div v-if="store.wizardGatewayDone" class="flex items-center gap-3 px-4 py-4 rounded-xl bg-emerald-50 border border-emerald-200 text-emerald-700">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span class="text-[13px] font-medium">网关已启动，一切就绪！</span>
            </div>

            <!-- 错误 -->
            <div v-if="store.wizardError" class="flex items-start gap-2 px-4 py-3 rounded-xl bg-red-50 border border-red-200 text-[12px] text-red-600">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="mt-px shrink-0">
                <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
              <span>{{ store.wizardError }}</span>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-end px-6 py-4 border-t border-[#f0ecfa] bg-[#faf9ff] shrink-0">
            <button
              v-if="store.wizardGatewayDone"
              type="button"
              class="px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#22c55e_0%,#16a34a_100%)] shadow-[0_2px_8px_rgba(34,197,94,0.2)]"
              @click="goToChat()"
            >
              开始使用
            </button>
            <button
              v-else-if="store.wizardError && !store.wizardRunning && !store.wizardStartingGateway"
              type="button"
              class="px-5 py-2 text-[13px] font-medium rounded-[8px] cursor-pointer transition border border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12"
              @click="handleStart()"
            >
              重试
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overlay-enter-active,
.overlay-leave-active {
  transition: opacity 0.2s ease;
}
.overlay-enter-active > div:last-child,
.overlay-leave-active > div:last-child {
  transition: transform 0.2s ease, opacity 0.2s ease;
}
.overlay-enter-from,
.overlay-leave-to {
  opacity: 0;
}
.overlay-enter-from > div:last-child {
  transform: scale(0.96) translateY(12px);
  opacity: 0;
}
.overlay-leave-to > div:last-child {
  transform: scale(0.96) translateY(12px);
  opacity: 0;
}
</style>
