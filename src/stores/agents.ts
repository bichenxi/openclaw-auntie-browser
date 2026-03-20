import { defineStore } from 'pinia'
import type { WizardPrompt } from '@/api/agents'

export const useAgentsStore = defineStore('agents', () => {
  // 向导可见性
  const wizardVisible = ref(false)
  // 向导运行中
  const wizardRunning = ref(false)
  // 当前 prompt
  const wizardPrompt = ref<WizardPrompt | null>(null)
  // 历史记录
  const wizardHistory = ref<Array<{ question: string; answer: string }>>([])
  // 输入值
  const wizardInputValue = ref('')
  // 错误信息
  const wizardError = ref<string | null>(null)
  // 退出码
  const wizardExitCode = ref<number | null>(null)
  // 正在启动网关
  const wizardStartingGateway = ref(false)
  // 完成
  const wizardDone = ref(false)
  // 当前添加的 work 名称
  const currentWork = ref('')
  // 当前正在编辑的智能体 work 名称
  const editingWork = ref('')

  function openWizard(work: string) {
    currentWork.value = work
    wizardVisible.value = true
    wizardRunning.value = false
    wizardPrompt.value = null
    wizardHistory.value = []
    wizardInputValue.value = ''
    wizardError.value = null
    wizardExitCode.value = null
    wizardStartingGateway.value = false
    wizardDone.value = false
  }

  function closeWizard() {
    wizardVisible.value = false
    wizardRunning.value = false
    wizardPrompt.value = null
    wizardHistory.value = []
    wizardInputValue.value = ''
    wizardError.value = null
    wizardExitCode.value = null
    wizardStartingGateway.value = false
    wizardDone.value = false
    currentWork.value = ''
  }

  return {
    wizardVisible,
    wizardRunning,
    wizardPrompt,
    wizardHistory,
    wizardInputValue,
    wizardError,
    wizardExitCode,
    wizardStartingGateway,
    wizardDone,
    currentWork,
    editingWork,
    openWizard,
    closeWizard,
  }
})
