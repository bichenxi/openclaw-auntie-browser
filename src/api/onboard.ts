import { invoke } from '@tauri-apps/api/core'

export interface OnboardParams {
  auth_choice: string
  api_key: string
  custom_base_url?: string
  custom_model_id?: string
}

export async function runOnboard(params: OnboardParams): Promise<void> {
  await invoke('run_onboard', { params })
}

/** 启动交互式 openclaw onboard（PTY 内嵌终端） */
export async function startOnboardPty(): Promise<void> {
  await invoke('start_onboard_pty')
}

/** 向 PTY 进程 stdin 写入（用户按键/输入） */
export async function writeOnboardStdin(data: string): Promise<void> {
  await invoke('write_onboard_stdin', { data })
}

/** 结束 PTY 进程 */
export async function killOnboardPty(): Promise<void> {
  await invoke('kill_onboard_pty')
}

export async function isOnboardPtyRunning(): Promise<boolean> {
  return invoke<boolean>('is_onboard_pty_running')
}

// ─── 卡片向导（跨平台 PTY + 屏幕解析）──────────────────────────────────────

export interface WizardPrompt {
  prompt_type: 'confirm' | 'select' | 'input' | 'password' | 'info' | 'done'
  question: string
  options: string[]
  selected: number
}

export async function startOnboardWizard(): Promise<void> {
  await invoke('start_onboard_wizard')
}

export async function wizardSendKey(action: string): Promise<void> {
  await invoke('wizard_send_key', { action })
}

export async function killOnboardWizard(): Promise<void> {
  await invoke('kill_onboard_wizard')
}

export async function isOnboardWizardRunning(): Promise<boolean> {
  return invoke<boolean>('is_onboard_wizard_running')
}
