<script setup lang="ts">
import { Terminal } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
import 'xterm/css/xterm.css'
import { useOnboardStore } from '@/stores/onboard'
import { useInstallerStore } from '@/stores/installer'
import { startOnboardPty, writeOnboardStdin, killOnboardPty } from '@/api/onboard'
import { listen } from '@tauri-apps/api/event'

const store = useOnboardStore()
const installerStore = useInstallerStore()
const terminalContainer = ref<HTMLElement | null>(null)
let term: Terminal | null = null
let fitAddon: FitAddon | null = null
let unlistens: Array<() => void> = []
let resizeObserver: ResizeObserver | null = null
const starting = ref(false)

function initTerminal() {
  if (!terminalContainer.value || term) return
  term = new Terminal({
    cursorBlink: true,
    fontSize: 12,
    fontFamily: 'Menlo, Monaco, "Courier New", monospace',
    theme: {
      background: '#1a1030',
      foreground: '#e2e8f0',
      cursor: '#5f47ce',
      cursorAccent: '#1a1030',
    },
  })
  fitAddon = new FitAddon()
  term.loadAddon(fitAddon)
  term.open(terminalContainer.value)
  fitAddon.fit()

  term.onData((data) => {
    writeOnboardStdin(data).catch(() => {})
  })

  resizeObserver = new ResizeObserver(() => {
    fitAddon?.fit()
  })
  resizeObserver.observe(terminalContainer.value)
}

function disposeTerminal() {
  if (resizeObserver && terminalContainer.value) {
    resizeObserver.unobserve(terminalContainer.value)
    resizeObserver = null
  }
  if (term) {
    term.dispose()
    term = null
  }
  fitAddon = null
}

function startListeners() {
  listen<{ data: string }>('onboard:pty_output', (e) => {
    if (term) term.write(e.payload.data)
  }).then((fn) => unlistens.push(fn))

  listen<{ code: number }>('onboard:pty_exited', (e) => {
    store.ptyRunning = false
    store.ptyExitCode = e.payload.code
    if (e.payload.code === 0) {
      installerStore.completeOnboard()
    }
  }).then((fn) => unlistens.push(fn))
}

function stopListeners() {
  for (const fn of unlistens) fn()
  unlistens = []
}

watch(
  () => store.visible,
  (visible) => {
    if (visible) {
      nextTick(() => initTerminal())
    } else {
      disposeTerminal()
    }
  },
)

onMounted(() => {
  startListeners()
  if (store.visible) nextTick(() => initTerminal())
})
onUnmounted(() => {
  stopListeners()
  disposeTerminal()
})

async function handleStart() {
  starting.value = true
  store.ptyError = null
  store.ptyExitCode = null
  try {
    await startOnboardPty()
    store.ptyRunning = true
  } catch (e: any) {
    store.ptyError = e?.message ?? String(e)
  } finally {
    starting.value = false
  }
}

async function handleKill() {
  try {
    await killOnboardPty()
    store.ptyRunning = false
  } catch (_) {}
}

function handleClose() {
  if (store.ptyRunning) {
    killOnboardPty().catch(() => {})
  }
  store.close()
}

const exitedSuccess = computed(() => store.ptyExitCode === 0)
const exitedWithError = computed(() => store.ptyExitCode !== null && store.ptyExitCode !== 0)
</script>

<template>
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="store.visible"
        class="fixed inset-0 z-[9999] flex items-center justify-center"
      >
        <div
          class="absolute inset-0 bg-black/40 backdrop-blur-sm"
          @click="!store.ptyRunning && handleClose()"
        />

        <div class="relative w-full max-w-[720px] mx-4 bg-white rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">

          <div class="flex items-center justify-between px-6 py-4 border-b border-[#f0ecfa] shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-[9px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow">
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                </svg>
              </div>
              <div>
                <div class="text-[15px] font-bold text-[#1f1f2e]">OpenClaw 初始化</div>
                <div class="text-[11px] text-[#9b8ec4]">内嵌终端 · 与 openclaw onboard TUI 实时交互</div>
              </div>
            </div>
            <button
              v-if="!store.ptyRunning"
              class="w-7 h-7 flex-center rounded-lg text-[#9b8ec4] hover:bg-[#f5f3ff] hover:text-secondary transition cursor-pointer bg-transparent border-none"
              @click="handleClose()"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <div class="px-6 py-3 bg-[#faf9ff] border-b border-[#f0ecfa] shrink-0">
            <p class="text-[12px] text-[#6b5f8a] m-0">
              点击「开始」后，将在此处运行 <code class="bg-[#f0ecfa] px-1.5 py-px rounded text-[11px]">openclaw onboard</code>，直接在此终端内用键盘操作即可。
            </p>
          </div>

          <div class="flex-1 min-h-0 p-4 flex flex-col">
            <div ref="terminalContainer" class="flex-1 min-h-[320px] w-full rounded-xl overflow-hidden bg-[#1a1030]" />

            <div v-if="store.ptyError" class="mt-3 flex items-start gap-2 px-4 py-3 rounded-xl bg-red-50 border border-red-200 text-[12px] text-red-600">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="mt-px shrink-0">
                <circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" />
              </svg>
              <span>{{ store.ptyError }}</span>
            </div>

            <div v-if="exitedSuccess" class="mt-3 flex items-center gap-2 px-4 py-3 rounded-xl bg-emerald-50 border border-emerald-200 text-[12px] text-emerald-700">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              初始化已完成（退出码 0）
            </div>
            <div v-else-if="exitedWithError" class="mt-3 flex items-center gap-2 px-4 py-3 rounded-xl bg-amber-50 border border-amber-200 text-[12px] text-amber-700">
              <span>进程已结束，退出码 {{ store.ptyExitCode }}</span>
            </div>
          </div>

          <div class="flex items-center justify-between px-6 py-4 border-t border-[#f0ecfa] bg-[#faf9ff] shrink-0">
            <div />
            <div class="flex items-center gap-2">
              <button
                v-if="!store.ptyRunning && store.ptyExitCode === null && !store.ptyError"
                class="px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] hover:shadow-[0_4px_12px_rgba(95,71,206,0.3)] disabled:opacity-50 disabled:cursor-not-allowed"
                :disabled="starting"
                @click="handleStart"
              >
                {{ starting ? '启动中…' : '开始' }}
              </button>
              <button
                v-if="store.ptyRunning"
                class="px-5 py-2 text-[13px] font-medium rounded-[8px] cursor-pointer transition border border-red-200 text-red-600 bg-red-50 hover:bg-red-100"
                @click="handleKill"
              >
                结束
              </button>
              <button
                v-if="store.ptyExitCode !== null || store.ptyError"
                class="px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#22c55e_0%,#16a34a_100%)] shadow-[0_2px_8px_rgba(34,197,94,0.2)]"
                @click="handleClose()"
              >
                完成
              </button>
            </div>
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

:deep(.xterm) {
  padding: 8px;
  height: 100%;
}
:deep(.xterm-viewport) {
  overflow-y: auto !important;
}
</style>
