import { invoke } from '@tauri-apps/api/core'

export interface WizardPrompt {
  prompt_type: string
  question: string
  options: string[]
  selected: number
  checked: number[]
  error?: string
  /** input 类型：TUI 里已填入的默认/当前值（去掉光标字符后） */
  current_value?: string
}

export interface AgentInfo {
  name: string
  workspace: string
  description?: string
}

/**
 * 列出所有已配置的智能体
 */
export async function listAgents(): Promise<AgentInfo[]> {
  return invoke<AgentInfo[]>('list_agents')
}

/** 列出智能体目录下已存在的文件 */
export async function listAgentFiles(work: string): Promise<string[]> {
  return invoke<string[]>('list_agent_files', { work })
}

/** 读取智能体文件内容，不存在返回空字符串 */
export async function readAgentFile(work: string, filename: string): Promise<string> {
  return invoke<string>('read_agent_file', { work, filename })
}

/** 写入智能体文件内容 */
export async function writeAgentFile(work: string, filename: string, content: string): Promise<void> {
  return invoke('write_agent_file', { work, filename, content })
}

/**
 * 启动 agents add 向导（跨平台 PTY + 屏幕解析）
 */
export async function startAgentAddWizard(work: string): Promise<void> {
  return invoke('start_agent_add_wizard', { work })
}

/**
 * 发送单个按键到向导
 */
export async function wizardSendKey(action: string): Promise<void> {
  return invoke('agent_wizard_send_key', { action })
}

/**
 * 批量发送按键到向导
 */
export async function wizardSendKeys(actions: string[]): Promise<void> {
  return invoke('agent_wizard_send_keys', { actions })
}

/**
 * 终止向导进程
 */
export async function killAgentWizard(): Promise<void> {
  return invoke('kill_agent_wizard')
}

/**
 * 检查向导是否在运行
 */
export async function isAgentWizardRunning(): Promise<boolean> {
  return invoke('is_agent_wizard_running')
}
