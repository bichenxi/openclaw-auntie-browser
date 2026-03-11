import { invoke } from '@tauri-apps/api/core'

export interface OpenclawInstallStatus {
  /** npm 包已安装（标记文件存在） */
  npm_installed: boolean
  /** onboard 已完成（~/.openclaw/openclaw.json 存在） */
  onboarded: boolean
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
