<script setup lang="ts">
import { useAgentsStore } from '@/stores/agents'
import {
  startAgentAddWizard,
  wizardSendKey,
  wizardSendKeys,
  killAgentWizard,
  type WizardPrompt,
} from '@/api/agents'
import { listen } from '@tauri-apps/api/event'

const store = useAgentsStore()
const unlistens = ref<Array<() => void>>([])
const starting = ref(false)
const sending = ref(false)
const waitingNext = ref(false)
let sendingTimer: ReturnType<typeof setTimeout> | null = null

/** multiselect 本地光标与勾选状态 */
const msCursor = ref(0)
const msChecked = ref<number[]>([])
const msQuestion = ref('')

/** select 本地光标 */
const selCursor = ref(0)
const selQuestion = ref('')

/** 调试面板 */
const showRaw = ref(false)
const rawTab = ref<'screen' | 'parsed' | 'logs'>('screen')
const screenText = ref('')
const screenCursorRow = ref(0)
const diagLogs = ref<string[]>([])

// ─── i18n ──────────────────────────────────────────────────────────────────

const i18nMap: Record<string, string> = {
  'Workspace directory': '工作区目录',
  'Copy auth profiles from "main"?': '从 "main" 复制认证配置？',
  'Configure model/auth for this agent now?': '现在配置此智能体的模型/认证？',
  'Configure chat channels now?': '现在配置消息通道？',

  // 通用
  'Yes': '是',
  'No': '否',
  'Skip': '跳过',
  'Skip for now': '暂时跳过',

  // onboard 相关（agents add 可能复用这些问题）
  'Model/auth provider': '模型 / 认证提供商',
  'Onboarding mode': '初始化模式',
  'Config handling': '配置处理方式',
  'Select channel (QuickStart)': '选择消息通道（快速配置）',
  'Default model': '默认模型',
  'Enable hooks?': '启用钩子？',
  'Configure skills now? (recommended)': '现在配置技能？（推荐）',
  'QuickStart (Configure details later via openclaw configure.)':
    '快速配置（稍后通过 openclaw configure 调整详情）',
  'Manual': '手动配置',
  'Onboarding complete!': '初始化完成！',
}

const i18nPrefixMap: Array<[string, string]> = [
  ['Local gateway (this machine)', '本地网关（本机）'],
  ['Remote gateway', '远程网关'],
  ['Keep current (', '保持当前（'],
  ['Onboarding complete', '初始化完成'],
]

function t(text: string): string {
  const trimmed = text.trim()
  const exact = i18nMap[trimmed]
  if (exact) return exact
  for (const [prefix, zhPrefix] of i18nPrefixMap) {
    if (trimmed.startsWith(prefix)) return zhPrefix + trimmed.slice(prefix.length)
  }
  return text
}

// ─── 锁控 ──────────────────────────────────────────────────────────────────

function unlockSending() {
  sending.value = false
  if (sendingTimer) { clearTimeout(sendingTimer); sendingTimer = null }
}

function lockSendingWithTimeout(ms = 500) {
  sending.value = true
  if (sendingTimer) clearTimeout(sendingTimer)
  sendingTimer = setTimeout(() => { sending.value = false }, ms)
}

// ─── 事件监听 ───────────────────────────────────────────────────────────────

function startListeners() {
  listen<WizardPrompt>('agent_wizard:prompt', (e) => {
    const prev = store.wizardPrompt
    const next = e.payload

    const isSamePrompt = prev
      && prev.prompt_type === next.prompt_type
      && prev.question === next.question
      && JSON.stringify(prev.options) === JSON.stringify(next.options)

    if (prev && prev.prompt_type !== 'done' && !isSamePrompt) {
      let answer = ''
      if (prev.prompt_type === 'confirm') answer = prev.options[prev.selected] ?? ''
      else if (prev.prompt_type === 'select') answer = prev.options[selCursor.value] ?? ''
      else if (prev.prompt_type === 'multiselect')
        answer = msChecked.value.map(idx => prev.options[idx]).filter(Boolean).join(', ')
      else if (prev.prompt_type === 'input' || prev.prompt_type === 'password') answer = '***'
      store.wizardHistory.push({ question: prev.question, answer: answer || '...' })
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
      // input 类型：用 TUI 里的当前值预填，方便用户直接回车确认或修改
      if ((next.prompt_type === 'input') && next.current_value) {
        store.wizardInputValue = next.current_value
      } else {
        store.wizardInputValue = ''
      }
    }

    unlockSending()
  }).then((fn) => unlistens.value.push(fn))

  listen<string>('agent_wizard:raw-data', (e) => {
    diagLogs.value.push(e.payload)
    if (diagLogs.value.length > 100) diagLogs.value.shift()
  }).then((fn) => unlistens.value.push(fn))

  listen<{ text: string; cursor_row: number }>('agent_wizard:screen', (e) => {
    screenText.value = e.payload.text
    screenCursorRow.value = e.payload.cursor_row
  }).then((fn) => unlistens.value.push(fn))

  listen<{ code: number }>('agent_wizard:exited', (e) => {
    store.wizardRunning = false
    store.wizardExitCode = e.payload.code
    waitingNext.value = false
    unlockSending()
    if (e.payload.code === 0) {
      store.wizardDone = true
    } else {
      store.wizardError = `进程退出码 ${e.payload.code}`
    }
  }).then((fn) => unlistens.value.push(fn))
}

function stopListeners() {
  unlistens.value.forEach((fn) => fn())
  unlistens.value = []
  unlockSending()
}

watch(
  () => store.wizardVisible,
  (visible) => {
    if (visible) startListeners()
    else stopListeners()
  },
)

onMounted(() => {
  if (store.wizardVisible) startListeners()
})
onUnmounted(stopListeners)

// ─── 操作 ───────────────────────────────────────────────────────────────────

async function handleStart() {
  starting.value = true
  store.wizardError = null
  store.wizardExitCode = null
  store.wizardPrompt = null
  store.wizardHistory = []
  diagLogs.value = []
  screenText.value = ''

  try {
    await startAgentAddWizard(store.currentWork)
    store.wizardRunning = true
  } catch (e: unknown) {
    store.wizardError = (e as Error)?.message ?? String(e)
  } finally {
    starting.value = false
  }
}

function sendNav(dir: 'up' | 'down') {
  const prompt = store.wizardPrompt
  if (!prompt || prompt.prompt_type !== 'multiselect') return
  const len = prompt.options.length
  if (len === 0) return
  if (dir === 'up') msCursor.value = (msCursor.value - 1 + len) % len
  else msCursor.value = (msCursor.value + 1) % len
  wizardSendKey(dir).catch(() => {})
}

function sendToggle() {
  if (sending.value) return
  const idx = msCursor.value
  const pos = msChecked.value.indexOf(idx)
  if (pos >= 0) msChecked.value.splice(pos, 1)
  else msChecked.value.push(idx)
  lockSendingWithTimeout(500)
  wizardSendKey('space').catch(() => { unlockSending() })
}

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
    await wizardSendKey(choice === 0 ? 'left' : 'right')
    await wizardSendKey('enter')
  } catch {
    waitingNext.value = false
    unlockSending()
  }
}

async function answerSelect(index: number) {
  if (sending.value) return
  waitingNext.value = true
  lockSendingWithTimeout(800)
  const delta = index - selCursor.value
  const keys: string[] = []
  if (delta > 0) for (let i = 0; i < delta; i++) keys.push('down')
  else if (delta < 0) for (let i = 0; i < -delta; i++) keys.push('up')
  keys.push('enter')
  selCursor.value = index
  await wizardSendKeys(keys).catch(() => { waitingNext.value = false; unlockSending() })
}

async function answerInput() {
  const text = store.wizardInputValue.trim()
  const prompt = store.wizardPrompt
  if (!prompt) return
  waitingNext.value = true
  lockSendingWithTimeout(600)

  const originalValue = prompt.current_value ?? ''
  if (text === originalValue || text === '') {
    // 用户未修改（或清空），直接回车接受 TUI 默认值
    await wizardSendKey('enter').catch(() => { waitingNext.value = false; unlockSending() })
  } else {
    // 用户修改了值：Ctrl+A 定位行首，Ctrl+K 清到行尾，再输入新值+回车
    const actions = [
      'text:\x01', // Ctrl+A
      'text:\x0b', // Ctrl+K
      `submit:${text}`,
    ]
    await wizardSendKeys(actions).catch(() => { waitingNext.value = false; unlockSending() })
  }
}

function handleClose() {
  if (store.wizardRunning) killAgentWizard().catch(() => {})
  unlockSending()
  store.closeWizard()
}

function handleDone() {
  store.closeWizard()
}
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="store.wizardVisible"
        class="fixed inset-0 z-[9999] flex items-center justify-center"
      >
        <div class="absolute inset-0 bg-black/40 backdrop-blur-sm" />

        <div class="relative w-full max-w-[520px] mx-4 bg-white rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
          <!-- Header -->
          <div class="flex items-center justify-between px-6 py-4 border-b border-[#f0ecfa] shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-[9px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                  <circle cx="9" cy="7" r="4" />
                  <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                  <path d="M16 3.13a4 4 0 0 1 0 7.75" />
                </svg>
              </div>
              <div>
                <div class="text-[15px] font-bold text-[#1f1f2e]">添加智能体</div>
                <div class="text-[11px] text-[#9b8ec4]">
                  <span v-if="store.wizardDone">配置完成</span>
                  <span v-else-if="store.wizardRunning">正在配置 {{ store.currentWork }}</span>
                  <span v-else>openclaw agents add {{ store.currentWork }}</span>
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
            <template v-if="!store.wizardRunning && !store.wizardPrompt && store.wizardExitCode === null && !store.wizardDone">
              <div class="flex flex-col items-center gap-4 py-8">
                <img src="/logo.png" class="w-14 h-14 rounded-[12px] object-cover shadow-lg" alt="logo" />
                <p class="text-[13px] text-[#6b5f8a] text-center max-w-[360px] leading-relaxed">
                  将运行 <code class="bg-[#f0ecfa] px-1.5 py-px rounded text-[11px]">openclaw agents add {{ store.currentWork }}</code> 进行配置。
                </p>
                <button
                  class="px-6 py-2.5 text-[14px] font-medium text-white rounded-[10px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_10px_rgba(95,71,206,0.22)] hover:shadow-[0_4px_16px_rgba(95,71,206,0.32)] disabled:opacity-50 disabled:cursor-not-allowed"
                  :disabled="starting"
                  @click="handleStart"
                >
                  {{ starting ? '启动中…' : '开始添加' }}
                </button>
              </div>
            </template>

            <!-- 等待第一个 prompt -->
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
            <template v-else-if="store.wizardPrompt && (store.wizardRunning || store.wizardPrompt.prompt_type === 'done')">
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

              <!-- Multiselect -->
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
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                  {{ store.wizardPrompt.error }}
                </div>
                <div class="flex items-center gap-2">
                  <button type="button" class="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium rounded-lg border border-[#e8e2f4] bg-white text-[#4a4568] hover:border-secondary/30 hover:bg-secondary/5 cursor-pointer transition active:scale-95" @click="sendNav('up')">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="18 15 12 9 6 15" /></svg>上移
                  </button>
                  <button type="button" class="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium rounded-lg border border-[#e8e2f4] bg-white text-[#4a4568] hover:border-secondary/30 hover:bg-secondary/5 cursor-pointer transition active:scale-95" @click="sendNav('down')">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="6 9 12 15 18 9" /></svg>下移
                  </button>
                  <button type="button" :disabled="sending" class="flex items-center gap-1 px-3 py-1.5 text-[12px] font-medium rounded-lg border border-secondary/30 bg-secondary/6 text-secondary hover:bg-secondary/12 cursor-pointer transition active:scale-95 disabled:opacity-40" @click="sendToggle()">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12" /></svg>切换
                  </button>
                  <div class="flex-1" />
                  <button type="button" :disabled="sending" class="px-5 py-1.5 text-[12px] font-medium text-white rounded-lg cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_6px_rgba(95,71,206,0.18)] active:scale-95 disabled:opacity-40" @click="sendSubmit()">确认</button>
                </div>
              </div>

              <!-- Input / Password（question 为空时显示 loading） -->
              <div v-else-if="(store.wizardPrompt.prompt_type === 'input' || store.wizardPrompt.prompt_type === 'password') && !store.wizardPrompt.question.trim()" class="flex flex-col items-center gap-3 py-10">
                <span class="w-7 h-7 border-[2.5px] border-secondary border-t-transparent rounded-full animate-spin" />
                <span class="text-[13px] text-[#9b8ec4]">正在加载配置项…</span>
              </div>
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
                  :disabled="!store.wizardInputValue.trim() && !store.wizardPrompt?.current_value"
                  @click="answerInput()"
                >
                  确认
                </button>
              </div>

              <!-- Done -->
              <div v-else-if="store.wizardPrompt.prompt_type === 'done'" class="flex items-center gap-3 px-4 py-4 rounded-xl bg-emerald-50 border border-emerald-200 text-emerald-700">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12" /></svg>
                <span class="text-[13px] font-medium">{{ t(store.wizardPrompt.question) }}</span>
              </div>

              <!-- Info / 未知 -->
              <div v-else class="px-4 py-3 rounded-xl bg-[#faf9ff] border border-[#f0ecfa] text-[12px] text-[#6b5f8a]">
                {{ t(store.wizardPrompt.question) }}
              </div>
            </template>

            <!-- 完成卡片 -->
            <div v-if="store.wizardDone" class="flex items-center gap-3 px-4 py-4 rounded-xl bg-emerald-50 border border-emerald-200 text-emerald-700">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12" /></svg>
              <span class="text-[13px] font-medium">智能体 "{{ store.currentWork }}" 已成功添加！</span>
            </div>

            <!-- TUI 调试面板 -->
            <div v-if="showRaw" class="bg-[#1a1030] rounded-xl overflow-hidden">
              <div class="flex items-center gap-0 border-b border-[#2a2040]">
                <button type="button" class="px-4 py-2 text-[12px] font-sans font-medium cursor-pointer transition border-none" :class="rawTab === 'screen' ? 'bg-[#2a2040] text-[#a78bfa]' : 'bg-transparent text-[#6b5f8a] hover:text-[#9b8ec4]'" @click="rawTab = 'screen'">终端全文</button>
                <button type="button" class="px-4 py-2 text-[12px] font-sans font-medium cursor-pointer transition border-none" :class="rawTab === 'parsed' ? 'bg-[#2a2040] text-[#a78bfa]' : 'bg-transparent text-[#6b5f8a] hover:text-[#9b8ec4]'" @click="rawTab = 'parsed'">解析结果</button>
                <button type="button" class="px-4 py-2 text-[12px] font-sans font-medium cursor-pointer transition border-none" :class="rawTab === 'logs' ? 'bg-[#2a2040] text-[#a78bfa]' : 'bg-transparent text-[#6b5f8a] hover:text-[#9b8ec4]'" @click="rawTab = 'logs'">诊断日志</button>
              </div>
              <div v-if="rawTab === 'screen'" class="p-4 font-mono text-[12px] leading-[1.7] overflow-auto max-h-[300px]">
                <div class="text-[11px] text-[#6b5f8a] mb-2 font-sans">cursor_row: {{ screenCursorRow }}</div>
                <pre class="m-0 whitespace-pre text-green-400">{{ screenText || '（等待终端输出…）' }}</pre>
              </div>
              <div v-else-if="rawTab === 'logs'" class="p-4 font-mono text-[12px] leading-[1.7] overflow-auto max-h-[300px] text-[#9b8ec4]">
                <div v-for="(log, i) in diagLogs" :key="i" class="mb-1">{{ log }}</div>
                <div v-if="!diagLogs.length" class="text-[#6b5f8a]">（无诊断日志）</div>
              </div>
              <div v-else-if="rawTab === 'parsed' && store.wizardPrompt" class="p-4 font-mono text-[13px] leading-[1.8]">
                <div class="text-green-400">
                  <div><span class="text-[#9b8ec4]">type: </span>{{ store.wizardPrompt.prompt_type }}</div>
                  <div><span class="text-[#9b8ec4]">question: </span>{{ store.wizardPrompt.question }}</div>
                  <div v-for="(opt, i) in store.wizardPrompt.options" :key="i" class="pl-4"><span class="text-[#6b5f8a]">{{ i }}. </span>{{ opt }}</div>
                </div>
              </div>
              <div v-else class="p-4 text-[12px] text-[#6b5f8a] font-sans">（暂无数据）</div>
            </div>

            <!-- 错误 -->
            <div v-if="store.wizardError" class="flex items-start gap-2 px-4 py-3 rounded-xl bg-red-50 border border-red-200 text-[12px] text-red-600">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="mt-px shrink-0"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
              <span>{{ store.wizardError }}</span>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-end px-6 py-4 border-t border-[#f0ecfa] bg-[#faf9ff] shrink-0">
            <button
              v-if="store.wizardDone"
              type="button"
              class="flex items-center gap-2 px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#22c55e_0%,#16a34a_100%)] shadow-[0_2px_8px_rgba(34,197,94,0.2)]"
              @click="handleDone()"
            >
              完成
            </button>
            <button
              v-else-if="store.wizardError && !store.wizardRunning"
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
.overlay-leave-to { opacity: 0; }
.overlay-enter-from > div:last-child {
  transform: scale(0.96) translateY(12px);
  opacity: 0;
}
.overlay-leave-to > div:last-child {
  transform: scale(0.96) translateY(12px);
  opacity: 0;
}
</style>
