import { invoke } from '@tauri-apps/api/core'

export interface OpenclawInstallStatus {
  /** npm 包已安装（标记文件存在） */
  npm_installed: boolean
  /** onboard 已完成（~/.openclaw/openclaw.json 存在） */
  onboarded: boolean
}

export interface EnvironmentInfo {
  node_version: string | null
  npm_version: string | null
  git_version: string | null
  has_nvm: boolean
  has_fnm: boolean
  strategy: string
  /** Windows: 用户主目录含非 ASCII 字符（中文用户名） */
  unicode_home_warning: boolean
  /** Windows: 已解析的 8.3 短路径，null 表示无法自动修正 */
  safe_home: string | null
}

/** 开始安装（检测环境 → npm install -g openclaw） */
export async function startInstall(): Promise<void> {
  await invoke('start_install')
}

/** 取消正在进行的安装 */
export async function cancelInstall(): Promise<void> {
  await invoke('cancel_install')
}

/** 检测 OpenClaw 安装状态 */
export async function checkOpenclawInstalled(): Promise<OpenclawInstallStatus> {
  return invoke<OpenclawInstallStatus>('check_openclaw_installed')
}

/** 检测当前系统环境（Node.js / npm / Git / 路径等） */
export async function detectEnvironment(): Promise<EnvironmentInfo> {
  return invoke<EnvironmentInfo>('detect_environment')
}
