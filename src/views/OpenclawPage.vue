<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useTabsStore } from '@/stores/tabs'
import { useSettingsStore } from '@/stores/settings'
import { useOpenclawStore } from '@/stores/openclaw'
import {
  checkOpenclawAlive,
  openclawSendV1,
  type OpenclawV1Params,
} from '@/api/openclaw'

const store = useTabsStore()
const settings = useSettingsStore()
const ocStore = useOpenclawStore()

const { messages, sending, sendError } = storeToRefs(ocStore)

const messagesEl = ref<HTMLElement | null>(null)

function scrollToBottom() {
  nextTick(() => {
    if (messagesEl.value) {
      messagesEl.value.scrollTop = messagesEl.value.scrollHeight
    }
  })
}

const openclawRunning = ref(false)

async function refreshStatus() {
  openclawRunning.value = await checkOpenclawAlive(settings.baseUrl || undefined)
}

// OpenClaw 自身连接状态（ping 18789）
const statusInfo = computed(() => {
  if (!openclawRunning.value) return { label: '未连接', color: 'gray', pulse: false }
  if (sending.value) return { label: '思考中', color: 'purple', pulse: true }
  return { label: '已连接', color: 'green', pulse: false }
})

const inputText = ref('')

async function send() {
  const text = inputText.value.trim()
  if (!text || sending.value) return
  sendError.value = ''
  messages.value.push({ type: 'user', text, streaming: false })
  inputText.value = ''
  scrollToBottom()
  sending.value = true
  try {
    const params: OpenclawV1Params = {
      input: text,
      stream: true,
    }
    if (settings.bearerToken) params.token = settings.bearerToken
    if (settings.sessionKey) params.session_key = settings.sessionKey
    if (settings.baseUrl) params.base_url = settings.baseUrl
    await openclawSendV1(params)
    // 正常情况由 stream-done 事件关闭 sending；invoke 失败时在 catch 里处理
  } catch (e) {
    sendError.value = String(e)
    sending.value = false
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}

const hasToken = computed(() => !!settings.bearerToken)

ocStore.startListeners()

onMounted(() => {
  refreshStatus()
  scrollToBottom()
  // 每 5 秒轮询一次状态
  const timer = setInterval(refreshStatus, 5000)
  onUnmounted(() => clearInterval(timer))
})

// 消息更新时滚动到底部
watch(messages, scrollToBottom, { deep: true })
</script>

<template>
  <div class="h-full flex flex-col bg-[linear-gradient(180deg,#f8f6ff_0%,#ffffff_100%)] overflow-hidden">
    <!-- 顶部状态栏 -->
    <div class="shrink-0 flex items-center justify-between px-5 py-3.5 border-b border-[#e8e2f4] bg-white">
      <div class="flex items-center gap-3">
        <img
          class="w-9 h-9 rounded-[10px] object-cover shadow-[0_2px_10px_rgba(0,0,0,0.12)] shrink-0"
          src="/logo.jpg"
          alt="logo"
        />
        <div class="flex flex-col">
          <span class="text-[16px] font-bold text-[#1f1f2e] leading-[1.2]">OpenClaw</span>
          <span class="text-[11px] text-[#9b8ec4] mt-px">AI 助手对话</span>
        </div>
      </div>
      <div
        class="flex items-center gap-[6px] px-3 py-[5px] rounded-[20px] text-[12px] font-medium border transition-all duration-500"
        :class="{
          'text-[#6b7280] bg-[rgba(107,114,128,0.08)] border-[rgba(107,114,128,0.18)]': statusInfo.color === 'gray',
          'text-[#16a34a] bg-[rgba(34,197,94,0.09)] border-[rgba(34,197,94,0.2)]': statusInfo.color === 'green',
          'text-[#7c5cfc] bg-secondary/8 border-secondary/22': statusInfo.color === 'purple',
          'text-[#d97706] bg-[rgba(245,158,11,0.09)] border-[rgba(245,158,11,0.22)]': statusInfo.color === 'amber',
        }"
      >
        <span
          class="w-[7px] h-[7px] rounded-full bg-current shrink-0"
          :class="{ 'animate-[pulse_1.5s_ease-in-out_infinite]': statusInfo.pulse }"
        />
        {{ statusInfo.label }}
      </div>
    </div>

    <!-- Token 状态提示 -->
    <div
      v-if="!hasToken"
      class="shrink-0 flex items-center gap-1 px-5 py-2 bg-[rgba(245,158,11,0.08)] border-b border-[rgba(245,158,11,0.2)] text-[12px] text-[#92400e]"
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
      <span>未配置 Bearer Token，请前往</span>
      <button
        type="button"
        class="bg-transparent border-none text-secondary text-[12px] cursor-pointer underline p-0"
        @click="store.switchToSpecialView('settings')"
      >设置页面</button>
      <span>进行配置</span>
    </div>

    <!-- 消息区 -->
    <div ref="messagesEl" class="oc-messages flex-1 overflow-y-auto p-5 flex flex-col gap-3">
      <!-- 空状态 -->
      <div v-if="messages.length === 0" class="flex-1 flex flex-col items-center justify-center gap-3 text-[#9b8ec4] py-10">
        <img
          class="w-[72px] h-[72px] rounded-[20px] object-cover shadow-[0_4px_20px_rgba(0,0,0,0.1)] opacity-85"
          src="/logo.jpg"
          alt="logo"
        />
        <p class="text-[14px] text-[#6b7280] m-0">向 OpenClaw 发送消息，开始对话</p>
        <p class="text-[12px] text-[#9b8ec4] m-0 text-center">支持自然语言指令，如「帮我搜索小红书上的旅游攻略」</p>
      </div>

      <!-- 消息列表 -->
      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="flex flex-col"
        :class="msg.type === 'user' ? 'items-end' : 'items-start'"
      >
        <template v-if="msg.type === 'user'">
          <div class="max-w-[75%] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] text-white px-3.5 py-2.5 rounded-[16px_16px_4px_16px] shadow-[0_2px_8px_rgba(95,71,206,0.25)]">
            <span class="text-[13px] leading-[1.6] text-white whitespace-pre-wrap break-words">{{ msg.text }}</span>
          </div>
        </template>
        <template v-else>
          <div
            class="max-w-[85%] flex flex-col gap-1 bg-white border border-[#e8e2f4] rounded-[4px_16px_16px_16px] px-3.5 py-2.5 shadow-[0_1px_4px_rgba(95,71,206,0.05)]"
            :class="msg.type === 'thought' ? 'border-l-[3px] border-l-[#7c5cfc]' : 'border-l-[3px] border-l-[#22c55e]'"
          >
            <span
              class="inline-flex items-center px-2 py-0.5 rounded-[4px] text-[10px] font-semibold tracking-[0.5px] uppercase self-start"
              :class="msg.type === 'thought'
                ? 'bg-secondary/10 text-secondary'
                : 'bg-[rgba(34,197,94,0.1)] text-[#16a34a]'"
            >
              {{ msg.type === 'thought' ? '思考' : '工具' }}
            </span>
            <span class="text-[13px] leading-[1.6] text-[#1f1f2e] whitespace-pre-wrap break-words">
              {{ msg.text }}<span v-if="msg.streaming" class="oc-cursor" />
            </span>
          </div>
        </template>
      </div>

      <!-- 等待首个 delta 到达前的打字动画 -->
      <div v-if="sending && !messages.some(m => m.streaming)" class="flex flex-col items-start">
        <div class="max-w-[85%] flex flex-col gap-1 bg-white border border-[#e8e2f4] border-l-[3px] border-l-[#7c5cfc] rounded-[4px_16px_16px_16px] px-3.5 py-2.5 shadow-[0_1px_4px_rgba(95,71,206,0.05)]">
          <span class="inline-flex items-center px-2 py-0.5 rounded-[4px] text-[10px] font-semibold tracking-[0.5px] uppercase self-start bg-secondary/10 text-secondary">思考中</span>
          <span class="oc-typing flex gap-1 py-1">
            <span /><span /><span />
          </span>
        </div>
      </div>
    </div>

    <!-- 错误提示 -->
    <div
      v-if="sendError"
      class="shrink-0 px-5 py-2.5 text-[12px] bg-[rgba(239,68,68,0.06)] border-t border-[rgba(239,68,68,0.15)] flex items-start gap-2"
    >
      <span class="text-[#dc2626] flex-1 leading-relaxed">{{ sendError }}</span>
      <button
        v-if="sendError.includes('502') || sendError.includes('401') || sendError.includes('403')"
        type="button"
        class="shrink-0 text-secondary text-[12px] underline bg-transparent border-none cursor-pointer p-0 leading-relaxed"
        @click="store.switchToSpecialView('settings')"
      >检查设置</button>
    </div>

    <!-- 输入区 -->
    <div class="shrink-0 px-5 py-4 bg-white border-t border-[#e8e2f4]">
      <textarea
        v-model="inputText"
        class="w-full px-3.5 py-2.5 text-[14px] font-[inherit] border-[1.5px] border-[#e8e2f4] rounded-[10px] resize-none outline-none box-border text-[#1f1f2e] leading-[1.5] transition placeholder-[#b8b0cc] focus:border-[#7c5cfc] focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)] disabled:opacity-60 disabled:cursor-not-allowed"
        placeholder="输入消息，Enter 发送，Shift+Enter 换行"
        rows="3"
        :disabled="sending"
        @keydown="handleKeydown"
      />
      <div class="flex items-center justify-between mt-2">
        <span
          v-if="settings.sessionKey"
          class="text-[11px] text-[#b8b0cc] max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap"
        >
          会话：{{ settings.sessionKey }}
        </span>
        <span v-else />
        <button
          type="button"
          class="flex items-center gap-1.5 px-[18px] py-2 bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] text-white border-none rounded-[8px] text-[13px] font-medium cursor-pointer transition shadow-[0_2px_8px_rgba(95,71,206,0.25)] hover:not-disabled:shadow-[0_4px_14px_rgba(95,71,206,0.35)] hover:not-disabled:-translate-y-px active:not-disabled:translate-y-0 disabled:opacity-50 disabled:cursor-not-allowed disabled:translate-y-0 disabled:shadow-none"
          :disabled="sending || !inputText.trim()"
          @click="send"
        >
          <svg v-if="!sending" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="22" y1="2" x2="11" y2="13" />
            <polygon points="22 2 15 22 11 13 2 9 22 2" />
          </svg>
          <span v-else class="send-loading" />
          {{ sending ? '发送中' : '发送' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 滚动条样式无法原子化 */
.oc-messages::-webkit-scrollbar {
  width: 4px;
}
.oc-messages::-webkit-scrollbar-track {
  background: transparent;
}
.oc-messages::-webkit-scrollbar-thumb {
  background: rgba(95, 71, 206, 0.15);
  border-radius: 2px;
}

/* 光标闪烁动画 */
.oc-cursor {
  display: inline-block;
  width: 2px;
  height: 1em;
  background: currentColor;
  margin-left: 2px;
  vertical-align: text-bottom;
  opacity: 0.7;
  animation: blink 0.9s step-end infinite;
}

@keyframes blink {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 0; }
}

/* 打字动画圆点 */
.oc-typing span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #9b8ec4;
  animation: typing-dot 1.2s ease-in-out infinite;
}
.oc-typing span:nth-child(2) { animation-delay: 0.2s; }
.oc-typing span:nth-child(3) { animation-delay: 0.4s; }

@keyframes typing-dot {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1); }
}

/* 发送中 loading spinner */
.send-loading {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: #ffffff;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* pulse 动画用于运行状态圆点 */
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
</style>
