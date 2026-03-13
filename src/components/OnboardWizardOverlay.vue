<script setup lang="ts">
import { useOnboardStore } from '@/stores/onboard'
import {
  startOnboardWizard,
  wizardSendKey,
  wizardSendKeys,
  killOnboardWizard,
  type WizardPrompt,
} from '@/api/onboard'
import { restartOpenclawGateway } from '@/api/gateway'
import { checkOpenclawAlive } from '@/api/openclaw'
import { useTabsStore } from '@/stores/tabs'
import { useInstallerStore } from '@/stores/installer'
import { useAutoSetup } from '@/composables/useAutoSetup'
import { listen } from '@tauri-apps/api/event'

const store = useOnboardStore()
const tabsStore = useTabsStore()
const installerStore = useInstallerStore()
const { autoSetup } = useAutoSetup()
const unlistens = ref<Array<() => void>>([])
const starting = ref(false)
const sending = ref(false)
/** 已提交答案，等待下一个 prompt */
const waitingNext = ref(false)
let sendingTimer: ReturnType<typeof setTimeout> | null = null
let pollTimer: ReturnType<typeof setInterval> | null = null

const autoConfiguring = ref(false)

/** multiselect 本地光标与勾选状态（不依赖后端 selected 检测） */
const msCursor = ref(0)
const msChecked = ref<number[]>([])
const msQuestion = ref('')

/** select 本地光标（TUI 循环导航，需用差量定位） */
const selCursor = ref(0)
const selQuestion = ref('')

/** 是否显示中文翻译 */
const showChinese = ref(true)
/** 是否展开 TUI 调试面板 */
const showRaw = ref(false)
/** 调试面板当前 Tab：screen=终端全文，parsed=解析结果 */
const rawTab = ref<'screen' | 'parsed'>('screen')
/** 完整的终端屏幕文本（后端每次解析都推送） */
const screenText = ref('')
const screenCursorRow = ref(0)

const i18nMap: Record<string, string> = {
  // ── 问题 ──
  'I understand this is personal-by-default and shared/multi-user use requires lock-down. Continue?':
    '我了解这是默认个人使用模式，共享/多用户需要额外锁定。是否继续？',
  'Onboarding mode': '初始化模式',
  'Config handling': '配置处理方式',
  'Model/auth provider': '模型 / 认证提供商',
  'MiniMax auth method': 'MiniMax 认证方式',
  'How do you want to provide this API key?': '如何提供此 API Key？',
  'Enter MiniMax China API key': '输入 MiniMax 中国区 API Key',
  'Default model': '默认模型',
  'Select channel (QuickStart)': '选择消息通道（快速配置）',
  'Search provider': '搜索引擎',
  'Configure skills now? (recommended)': '现在配置技能？（推荐）',
  'Enable hooks?': '启用钩子？',

  // ── 选项 ──
  'Yes': '是',
  'No': '否',
  'QuickStart (Configure details later via openclaw configure.)':
    '快速配置（稍后通过 openclaw configure 调整详情）',
  'Manual': '手动配置',
  'Use existing values': '使用现有值',
  'Update values': '更新值',
  'Reset': '重置',
  'Paste API key now (Stores the key directly in OpenClaw config)':
    '现在粘贴 API Key（直接存储到 OpenClaw 配置中）',
  'Use external secret provider': '使用外部密钥管理',
  'Skip for now': '暂时跳过',
  '🚀 boot-md (Run BOOT.md on gateway startup)': '🚀 boot-md（网关启动时运行 BOOT.md）',
  '📎 bootstrap-extra-files': '📎 bootstrap-extra-files（引导附加文件）',
  '📝 command-logger': '📝 command-logger（命令日志记录）',
  '💾 session-memory': '💾 session-memory（会话记忆）',

  'Keep current': '保持当前',
  'Enter model': '输入模型名称',
  'Gateway service already installed': '网关服务已安装',
  'Restart': '重启',
  'Reinstall': '重新安装',
  'Skip': '跳过',
  'How do you want to hatch your bot?': '如何孵化你的 Bot？',
  'Hatch in TUI (recommended)': '在 TUI 中孵化（推荐）',
  'Open the Web UI': '打开 Web UI',
  'Do this later': '稍后再说',
  'What do you want to set up?': '你想设置什么？',
  'Workspace directory': '工作区目录',

  // ── 通用 ──
  'OpenAI (Codex OAuth + API key)': 'OpenAI（Codex OAuth + API Key）',
  'MiniMax OAuth (Oauth plugin for MiniMax)': 'MiniMax OAuth（MiniMax OAuth 插件）',
  'Telegram (Bot API) (recommended · newcomer-friendly)':
    'Telegram（Bot API）（推荐 · 新手友好）',
  'Brave Search (Structured results · country/language/time filters)':
    'Brave Search（结构化结果 · 国家/语言/时间筛选）',
}

const i18nPrefixMap: Array<[string, string]> = [
  ['Local gateway (this machine)', '本地网关（本机）'],
  ['Remote gateway', '远程网关（仅信息）'],
  ['Keep current (', '保持当前（'],
]

function t(text: string): string {
  if (!showChinese.value) return text
  const trimmed = text.trim()
  const exact = i18nMap[trimmed]
  if (exact) return exact
  for (const [prefix, zhPrefix] of i18nPrefixMap) {
    if (trimmed.startsWith(prefix)) {
      return zhPrefix + trimmed.slice(prefix.length)
    }
  }
  return text
}

function unlockSending() {
  sending.value = false
  if (sendingTimer) { clearTimeout(sendingTimer); sendingTimer = null }
}

function lockSendingWithTimeout(ms = 500) {
  sending.value = true
  if (sendingTimer) clearTimeout(sendingTimer)
  sendingTimer = setTimeout(() => { sending.value = false }, ms)
}

function startListeners() {
  listen<WizardPrompt>('wizard:prompt', (e) => {
    const prev = store.wizardPrompt
    const next = e.payload

    const isSamePrompt = prev
      && prev.prompt_type === next.prompt_type
      && prev.question === next.question
      && JSON.stringify(prev.options) === JSON.stringify(next.options)

    if (prev && prev.prompt_type !== 'done' && !isSamePrompt) {
      let answer = ''
      if (prev.prompt_type === 'confirm') {
        answer = prev.options[prev.selected] ?? ''
      } else if (prev.prompt_type === 'select') {
        answer = prev.options[selCursor.value] ?? ''
      } else if (prev.prompt_type === 'multiselect') {
        answer = msChecked.value.map(idx => prev.options[idx]).filter(Boolean).join(', ')
      } else if (prev.prompt_type === 'input' || prev.prompt_type === 'password') {
        answer = '***'
      }
      store.wizardHistory.push({
        question: prev.question,
        answer: answer || '...',
      })
    }
    store.wizardPrompt = next
    waitingNext.value = false

    if (next.prompt_type === 'multiselect') {
      if (msQuestion.value !== next.question) {
        msCursor.value = 0
        msChecked.value = [...next.checked]
        msQuestion.value = next.question
      }
    } else if (next.prompt_type === 'select') {
      if (selQuestion.value !== next.question) {
        selCursor.value = next.selected
        selQuestion.value = next.question
      }
    } else if (!isSamePrompt) {
      store.wizardInputValue = ''
    }

    unlockSending()
  }).then((fn) => unlistens.value.push(fn))

  listen<{ text: string; cursor_row: number }>('wizard:screen', (e) => {
    screenText.value = e.payload.text
    screenCursorRow.value = e.payload.cursor_row
  }).then((fn) => unlistens.value.push(fn))

  listen<{ code: number }>('wizard:exited', (e) => {
    store.wizardRunning = false
    store.wizardExitCode = e.payload.code
    waitingNext.value = false
    unlockSending()
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
  unlockSending()
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

/** 导航键（up/down）不加锁，立即更新本地光标 */
function sendNav(dir: 'up' | 'down') {
  const prompt = store.wizardPrompt
  if (!prompt || prompt.prompt_type !== 'multiselect') return
  const len = prompt.options.length
  if (len === 0) return
  if (dir === 'up') {
    msCursor.value = (msCursor.value - 1 + len) % len
  } else {
    msCursor.value = (msCursor.value + 1) % len
  }
  wizardSendKey(dir).catch(() => {})
}

/** 切换当前光标项的勾选状态，本地立即更新 */
function sendToggle() {
  if (sending.value) return
  const idx = msCursor.value
  const pos = msChecked.value.indexOf(idx)
  if (pos >= 0) {
    msChecked.value.splice(pos, 1)
  } else {
    msChecked.value.push(idx)
  }
  lockSendingWithTimeout(500)
  wizardSendKey('space').catch(() => { unlockSending() })
}

/** 提交 multiselect */
function sendSubmit() {
  if (sending.value) return
  waitingNext.value = true
  lockSendingWithTimeout(500)
  wizardSendKey('enter').catch(() => { waitingNext.value = false; unlockSending() })
}

async function answerConfirm(choice: number) {
  if (sending.value) return
  waitingNext.value = true
  lockSendingWithTimeout(600)
  try {
    if (choice === 0) {
      await wizardSendKey('left')
    } else {
      await wizardSendKey('right')
    }
    await wizardSendKey('enter')
  } catch {
    waitingNext.value = false
    unlockSending()
  }
}

async function answerSelect(index: number) {
  if (sending.value) return
  const prompt = store.wizardPrompt
  if (!prompt) return
  waitingNext.value = true
  lockSendingWithTimeout(800)
  const delta = index - selCursor.value
  const keys: string[] = []
  if (delta > 0) {
    for (let i = 0; i < delta; i++) keys.push('down')
  } else if (delta < 0) {
    for (let i = 0; i < -delta; i++) keys.push('up')
  }
  keys.push('enter')
  selCursor.value = index
  await wizardSendKeys(keys).catch(() => { waitingNext.value = false; unlockSending() })
}

async function answerInput() {
  const text = store.wizardInputValue.trim()
  if (!text) return
  waitingNext.value = true
  lockSendingWithTimeout(600)
  await wizardSendKey(`submit:${text}`).catch(() => { waitingNext.value = false; unlockSending() })
}

async function startGateway() {
  store.wizardStartingGateway = true
  try {
    await restartOpenclawGateway()
  } catch (_) {
    // ignore
  }
  pollTimer = setInterval(async () => {
    const alive = await checkOpenclawAlive().catch(() => false)
    if (alive) {
      if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
      store.wizardGatewayDone = true
      store.wizardStartingGateway = false
      installerStore.completeOnboard()
    }
  }, 1500)
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
  unlockSending()
  screenText.value = ''
  screenCursorRow.value = 0
  store.closeWizard()
}

async function goToChat() {
  autoConfiguring.value = true
  await autoSetup()
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
          @click="handleClose()"
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
            <div class="flex items-center gap-1.5">
              <button
                class="h-7 px-2 flex-center rounded-lg text-[10px] font-medium transition cursor-pointer border-none"
                :class="showRaw ? 'bg-[#1a1030] text-green-400' : 'bg-transparent text-[#9b8ec4] hover:bg-[#f5f3ff]'"
                title="查看 TUI 原始数据"
                @click="showRaw = !showRaw"
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="4 17 10 11 4 5" />
                  <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
              </button>
              <button
                class="h-7 px-2 flex-center rounded-lg text-[10px] font-medium transition cursor-pointer border-none"
                :class="showChinese ? 'bg-secondary/10 text-secondary' : 'bg-transparent text-[#9b8ec4] hover:bg-[#f5f3ff]'"
                title="中英文切换"
                @click="showChinese = !showChinese"
              >
                {{ showChinese ? '中' : 'EN' }}
              </button>
              <button
                class="w-7 h-7 flex-center rounded-lg text-[#9b8ec4] hover:bg-[#f5f3ff] hover:text-secondary transition cursor-pointer bg-transparent border-none"
                @click="handleClose()"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          </div>

          <!-- Body -->
          <div class="flex-1 min-h-0 p-6 flex flex-col overflow-y-auto gap-4">
            <!-- 未启动：开始按钮 -->
            <template v-if="!store.wizardRunning && !store.wizardPrompt && store.wizardExitCode === null && !store.wizardStartingGateway && !store.wizardGatewayDone">
              <div class="flex flex-col items-center gap-4 py-8">
                <img src="/logo.png" class="w-14 h-14 rounded-[12px] object-cover shadow-lg" alt="logo" />
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

            <!-- 等待第一个 prompt 的 loading -->
            <div v-if="store.wizardRunning && !store.wizardPrompt" class="flex flex-col items-center gap-3 py-10">
              <span class="w-7 h-7 border-[2.5px] border-secondary border-t-transparent rounded-full animate-spin" />
              <span class="text-[13px] text-[#9b8ec4]">正在加载配置项…</span>
            </div>

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
                <div class="text-[12px] text-[#6b5f8a]">{{ t(item.question) }}</div>
                <div class="text-[13px] font-medium text-secondary">{{ t(item.answer) }}</div>
              </div>
            </div>

            <!-- 当前 prompt 卡片 -->
            <div v-if="waitingNext" class="flex flex-col items-center gap-3 py-8">
              <span class="w-6 h-6 border-[2.5px] border-secondary border-t-transparent rounded-full animate-spin" />
              <span class="text-[12px] text-[#9b8ec4]">正在处理…</span>
            </div>
            <template v-else-if="store.wizardPrompt && store.wizardRunning">
              <!-- Confirm -->
              <div v-if="store.wizardPrompt.prompt_type === 'confirm'" class="flex flex-col gap-3">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ t(store.wizardPrompt.question) }}</p>
                <div class="flex gap-3">
                  <button
                    v-for="(opt, i) in store.wizardPrompt.options"
                    :key="i"
                    type="button"
                    :disabled="sending"
                    class="flex-1 py-2.5 text-[13px] font-medium rounded-xl border cursor-pointer transition disabled:opacity-50"
                    :class="i === 0
                      ? 'border-secondary bg-secondary/8 text-secondary hover:bg-secondary/15'
                      : 'border-[#e8e2f4] text-[#6b5f8a] hover:border-secondary/30 hover:bg-secondary/5'"
                    @click="answerConfirm(i)"
                  >
                    {{ t(opt) }}
                  </button>
                </div>
              </div>

              <!-- Select -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'select'" class="flex flex-col gap-2">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ t(store.wizardPrompt.question) }}</p>
                <button
                  v-for="(opt, i) in store.wizardPrompt.options"
                  :key="i"
                  type="button"
                  :disabled="sending"
                  class="flex items-center gap-3 px-4 py-3 rounded-xl border text-left transition cursor-pointer disabled:opacity-50"
                  :class="store.wizardPrompt.selected === i
                    ? 'border-secondary bg-secondary/8 text-secondary'
                    : 'border-[#e8e2f4] bg-white text-[#4a4568] hover:border-secondary/30 hover:bg-secondary/5'"
                  @click="answerSelect(i)"
                >
                  <span class="w-4 h-4 rounded-full border-2 flex-center shrink-0" :class="store.wizardPrompt.selected === i ? 'border-secondary bg-secondary' : 'border-[#c4bdd8]'">
                    <span v-if="store.wizardPrompt.selected === i" class="w-1.5 h-1.5 rounded-full bg-white" />
                  </span>
                  <span class="text-[13px] font-medium">{{ t(opt) }}</span>
                </button>
              </div>

              <!-- Multiselect：本地光标/勾选 + 导航按钮 -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'multiselect'" class="flex flex-col gap-3">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ t(store.wizardPrompt.question) }}</p>

                <div class="flex flex-col rounded-xl border border-[#e8e2f4] overflow-hidden">
                  <div
                    v-for="(opt, i) in store.wizardPrompt.options"
                    :key="i"
                    class="flex items-center gap-3 px-4 py-2.5 transition-colors duration-100"
                    :class="[
                      msCursor === i ? 'bg-secondary/6' : 'bg-white',
                      i < store.wizardPrompt.options.length - 1 ? 'border-b border-[#f0ecfa]' : '',
                    ]"
                  >
                    <span class="w-3 text-[13px] font-bold shrink-0" :class="msCursor === i ? 'text-secondary' : 'text-transparent'">›</span>
                    <div class="w-4 h-4 rounded border-2 flex-center shrink-0 transition" :class="msChecked.includes(i) ? 'border-secondary bg-secondary' : 'border-[#c4bdd8]'">
                      <svg v-if="msChecked.includes(i)" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="4">
                        <polyline points="20 6 9 17 4 12" />
                      </svg>
                    </div>
                    <span class="text-[13px]" :class="msChecked.includes(i) ? 'text-secondary font-medium' : 'text-[#4a4568]'">{{ t(opt) }}</span>
                  </div>
                </div>

                <div v-if="store.wizardPrompt.error" class="text-[11px] text-red-500 font-medium flex items-center gap-1.5">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
                  </svg>
                  {{ store.wizardPrompt.error }}
                </div>

                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    class="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium rounded-lg border border-[#e8e2f4] bg-white text-[#4a4568] hover:border-secondary/30 hover:bg-secondary/5 cursor-pointer transition active:scale-95"
                    @click="sendNav('up')"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="18 15 12 9 6 15" /></svg>
                    上移
                  </button>
                  <button
                    type="button"
                    class="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium rounded-lg border border-[#e8e2f4] bg-white text-[#4a4568] hover:border-secondary/30 hover:bg-secondary/5 cursor-pointer transition active:scale-95"
                    @click="sendNav('down')"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="6 9 12 15 18 9" /></svg>
                    下移
                  </button>
                  <button
                    type="button"
                    :disabled="sending"
                    class="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium rounded-lg border border-secondary/30 bg-secondary/6 text-secondary hover:bg-secondary/12 cursor-pointer transition active:scale-95 disabled:opacity-40"
                    @click="sendToggle()"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12" /></svg>
                    切换
                  </button>
                  <div class="flex-1" />
                  <button
                    type="button"
                    :disabled="sending"
                    class="px-5 py-1.5 text-[12px] font-medium text-white rounded-lg cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_6px_rgba(95,71,206,0.18)] active:scale-95 disabled:opacity-40"
                    @click="sendSubmit()"
                  >
                    确认
                  </button>
                </div>
                <p class="text-[10px] text-[#9b8ec4] m-0">↑↓ 移动光标，「切换」勾选/取消，「确认」提交</p>
              </div>

              <!-- Input / Password -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'input' || store.wizardPrompt.prompt_type === 'password'" class="flex flex-col gap-3">
                <p class="text-[13px] font-medium text-[#4a4568] m-0">{{ t(store.wizardPrompt.question) }}</p>
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
                <span class="text-[13px] font-medium">{{ t(store.wizardPrompt.question) }}</span>
              </div>

              <!-- Info / 未知类型 -->
              <div v-else class="px-4 py-3 rounded-xl bg-[#faf9ff] border border-[#f0ecfa] text-[12px] text-[#6b5f8a]">
                {{ t(store.wizardPrompt.question) }}
              </div>
            </template>

            <!-- TUI 调试面板 -->
            <div
              v-if="showRaw"
              class="bg-[#1a1030] rounded-xl overflow-hidden"
            >
              <div class="flex items-center gap-0 border-b border-[#2a2040]">
                <button
                  type="button"
                  class="px-4 py-2 text-[12px] font-sans font-medium cursor-pointer transition border-none"
                  :class="rawTab === 'screen' ? 'bg-[#2a2040] text-[#a78bfa]' : 'bg-transparent text-[#6b5f8a] hover:text-[#9b8ec4]'"
                  @click="rawTab = 'screen'"
                >
                  终端全文
                </button>
                <button
                  type="button"
                  class="px-4 py-2 text-[12px] font-sans font-medium cursor-pointer transition border-none"
                  :class="rawTab === 'parsed' ? 'bg-[#2a2040] text-[#a78bfa]' : 'bg-transparent text-[#6b5f8a] hover:text-[#9b8ec4]'"
                  @click="rawTab = 'parsed'"
                >
                  解析结果
                </button>
              </div>

              <!-- 终端全文输出 -->
              <div v-if="rawTab === 'screen'" class="p-4 font-mono text-[12px] leading-[1.7] overflow-auto max-h-[400px]">
                <div class="text-[11px] text-[#6b5f8a] mb-2 font-sans">cursor_row: {{ screenCursorRow }}</div>
                <pre class="m-0 whitespace-pre text-green-400">{{ screenText || '（等待终端输出…）' }}</pre>
              </div>

              <!-- 解析后的 prompt 数据 -->
              <div v-else-if="rawTab === 'parsed' && store.wizardPrompt" class="p-4 font-mono text-[13px] leading-[1.8] overflow-x-auto">
                <div class="flex items-center justify-between mb-3">
                  <span class="text-[12px] text-[#9b8ec4] font-sans font-medium">解析结果</span>
                  <span class="text-[11px] px-2 py-0.5 rounded bg-[#2a2040] text-[#a78bfa] font-sans font-medium">{{ store.wizardPrompt.prompt_type }}</span>
                </div>
                <div class="text-green-400">
                  <div><span class="text-[#9b8ec4]">question: </span>{{ store.wizardPrompt.question }}</div>
                  <div><span class="text-[#9b8ec4]">selected: </span>{{ store.wizardPrompt.selected }}</div>
                  <div v-if="store.wizardPrompt.checked?.length"><span class="text-[#9b8ec4]">checked: </span>[{{ store.wizardPrompt.checked.join(', ') }}]</div>
                  <div v-if="store.wizardPrompt.error"><span class="text-red-400">error: </span>{{ store.wizardPrompt.error }}</div>
                  <div class="mt-2 text-[#9b8ec4]">options:</div>
                  <div v-for="(opt, i) in store.wizardPrompt.options" :key="i" class="pl-4">
                    <span class="text-[#6b5f8a]">{{ i }}. </span>{{ opt }}
                  </div>
                </div>
              </div>
              <div v-else class="p-4 text-[12px] text-[#6b5f8a] font-sans">（暂无解析数据）</div>
            </div>

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
              class="flex items-center gap-2 px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#22c55e_0%,#16a34a_100%)] shadow-[0_2px_8px_rgba(34,197,94,0.2)] disabled:opacity-70 disabled:cursor-not-allowed"
              :disabled="autoConfiguring"
              @click="goToChat()"
            >
              <span v-if="autoConfiguring" class="w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin" />
              {{ autoConfiguring ? '配置中...' : '开始使用' }}
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
