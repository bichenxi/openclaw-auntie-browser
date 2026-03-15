import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import type { EnvironmentInfo } from '@/api/installer'
import { detectEnvironment } from '@/api/installer'

export type StepStatus = 'pending' | 'running' | 'done' | 'error'

export interface InstallStep {
  id: string
  label: string
  status: StepStatus
}

const INITIAL_STEPS: InstallStep[] = [
  { id: 'install-node', label: '检测 / 安装 Node.js 环境', status: 'pending' },
  { id: 'install-openclaw', label: '安装 OpenClaw（npm install -g openclaw）', status: 'pending' },
  { id: 'onboard', label: '初始化配置（openclaw onboard）', status: 'pending' },
]

const MAX_LOG_LINES = 200

export const useInstallerStore = defineStore('installer', () => {
  const steps = ref<InstallStep[]>(INITIAL_STEPS.map((s) => ({ ...s })))
  const logs = ref<string[]>([])
  const installing = ref(false)
  const error = ref<string | null>(null)
  const done = ref(false)
  /** npm 包已安装（可能尚未 onboard） */
  const isInstalled = ref(false)
  /** openclaw onboard 已完成（openclaw.json 存在） */
  const isOnboarded = ref(false)
  /** npm 安装完成，等待用户执行 onboard（在安装向导内展示第三步） */
  const needOnboard = ref(false)

  /** 环境检测结果 */
  const envInfo = ref<EnvironmentInfo | null>(null)
  const envDetecting = ref(false)

  let unlistens: Array<() => void> = []

  function resetSteps() {
    steps.value = INITIAL_STEPS.map((s) => ({ ...s }))
    logs.value = []
    error.value = null
    done.value = false
    needOnboard.value = false
  }

  function startListeners() {
    if (unlistens.length > 0) return

    listen<{ step: string; status: string }>('installer:step', (e) => {
      const { step, status } = e.payload
      const found = steps.value.find((s) => s.id === step)
      if (found) found.status = status as StepStatus
    }).then((fn) => unlistens.push(fn))

    listen<{ line: string }>('installer:log', (e) => {
      logs.value.push(e.payload.line)
      if (logs.value.length > MAX_LOG_LINES) {
        logs.value.splice(0, logs.value.length - MAX_LOG_LINES)
      }
    }).then((fn) => unlistens.push(fn))

    listen('installer:need-onboard', () => {
      installing.value = false
      needOnboard.value = true
      const step = steps.value.find((s) => s.id === 'onboard')
      if (step) step.status = 'running'
    }).then((fn) => unlistens.push(fn))

    // 兼容旧版（future use）
    listen('installer:done', () => {
      installing.value = false
      done.value = true
    }).then((fn) => unlistens.push(fn))

    listen<{ step: string; message: string }>('installer:error', (e) => {
      installing.value = false
      error.value = e.payload.message
    }).then((fn) => unlistens.push(fn))
  }

  function stopListeners() {
    for (const fn of unlistens) fn()
    unlistens = []
  }

  async function detectEnv() {
    envDetecting.value = true
    try {
      envInfo.value = await detectEnvironment()
    } catch {
      envInfo.value = null
    } finally {
      envDetecting.value = false
    }
  }

  function completeOnboard() {
    const step = steps.value.find((s) => s.id === 'onboard')
    if (step) step.status = 'done'
    needOnboard.value = false
    isInstalled.value = true
    isOnboarded.value = true
    done.value = true
  }

  return {
    steps, logs, installing, error, done,
    isInstalled, isOnboarded, needOnboard,
    envInfo, envDetecting,
    resetSteps, completeOnboard, startListeners, stopListeners, detectEnv,
  }
})
