import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'

export type StepStatus = 'pending' | 'running' | 'done' | 'error'

export interface InstallStep {
  id: string
  label: string
  status: StepStatus
}

const INITIAL_STEPS: InstallStep[] = [
  { id: 'install-node', label: '检测 / 安装 Node.js 环境', status: 'pending' },
  { id: 'install-openclaw', label: '安装 OpenClaw（npm install -g openclaw）', status: 'pending' },
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

  let unlistens: Array<() => void> = []

  function resetSteps() {
    steps.value = INITIAL_STEPS.map((s) => ({ ...s }))
    logs.value = []
    error.value = null
    done.value = false
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

    // npm 安装完成，需要用户手动运行 openclaw onboard
    listen('installer:need-onboard', () => {
      installing.value = false
      isInstalled.value = true
      isOnboarded.value = false
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

  return {
    steps, logs, installing, error, done,
    isInstalled, isOnboarded,
    resetSteps, startListeners, stopListeners,
  }
})
