import { defineStore } from 'pinia'
import type { WizardPrompt } from '@/api/onboard'

/** PTY 浮层：内嵌终端，仅 Unix */
export const useOnboardStore = defineStore('onboard', () => {
  const visible = ref(false)
  const ptyRunning = ref(false)
  const ptyExitCode = ref<number | null>(null)
  const ptyError = ref<string | null>(null)

  function open() {
    visible.value = true
    ptyRunning.value = false
    ptyExitCode.value = null
    ptyError.value = null
  }

  function close() {
    visible.value = false
  }

  // ─── 卡片向导（跨平台 PTY + 屏幕解析 → prompt 事件驱动）──────────────────
  const wizardVisible = ref(false)
  const wizardRunning = ref(false)
  const wizardError = ref<string | null>(null)
  const wizardExitCode = ref<number | null>(null)
  /** 当前 prompt（后端通过事件推送） */
  const wizardPrompt = ref<WizardPrompt | null>(null)
  /** 已完成的 prompt 历史（展示在上方） */
  const wizardHistory = ref<Array<{ question: string; answer: string }>>([])
  /** 用户在 input/password 类型中输入的文本 */
  const wizardInputValue = ref('')
  /** onboard 成功后进入「启动网关」阶段 */
  const wizardStartingGateway = ref(false)
  const wizardGatewayDone = ref(false)

  function openWizard() {
    wizardVisible.value = true
    wizardRunning.value = false
    wizardError.value = null
    wizardExitCode.value = null
    wizardPrompt.value = null
    wizardHistory.value = []
    wizardInputValue.value = ''
    wizardStartingGateway.value = false
    wizardGatewayDone.value = false
  }

  function closeWizard() {
    wizardVisible.value = false
  }

  return {
    visible,
    ptyRunning,
    ptyExitCode,
    ptyError,
    open,
    close,
    wizardVisible,
    wizardRunning,
    wizardError,
    wizardExitCode,
    wizardPrompt,
    wizardHistory,
    wizardInputValue,
    wizardStartingGateway,
    wizardGatewayDone,
    openWizard,
    closeWizard,
  }
})
