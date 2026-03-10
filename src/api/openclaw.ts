import { invoke } from '@tauri-apps/api/core'

const DEFAULT_URL = 'ws://127.0.0.1:18789'

export async function openclawConnect(
  url?: string,
  token?: string,
): Promise<void> {
  await invoke('openclaw_connect', {
    url: url || DEFAULT_URL,
    token: token || null,
  })
}

export async function openclawSendChat(text: string): Promise<void> {
  await invoke('openclaw_send_chat', { text })
}

export async function openclawDisconnect(): Promise<void> {
  await invoke('openclaw_disconnect')
}

/** 启动 OpenClaw 子进程（Node 入口或 Sidecar）。需设 OPENCLAW_ENTRY 或用 pkg 打好 bin/openclaw */
export async function startOpenclawProcess(): Promise<void> {
  await invoke('start_openclaw')
}

/** 停止 OpenClaw 子进程 */
export async function stopOpenclawProcess(): Promise<void> {
  await invoke('stop_openclaw')
}

/** 是否正在运行 OpenClaw 子进程 */
export async function isOpenclawProcessRunning(): Promise<boolean> {
  return invoke<boolean>('is_openclaw_running')
}

/** 临时会话参数（携带本轮完整上下文） */
export interface OpenclawCompletionsParams {
  base_url?: string
  token?: string
  session_key?: string
  model?: string
  messages: Array<{ role: 'user' | 'assistant'; content: string }>
}

/** 临时会话：POST /v1/chat/completions，流式结果通过 temp-stream-item / temp-stream-done 事件推送 */
export async function openclawSendCompletions(params: OpenclawCompletionsParams): Promise<void> {
  await invoke('openclaw_send_completions', { params })
}


export interface OpenclawV1Params {
  base_url?: string
  token?: string
  session_key?: string
  model?: string
  input: string
  stream?: boolean
}

/** 通过 HTTP POST /v1/responses 发送输入，流式结果通过 stream-item 事件展示 */
export async function openclawSendV1(params: OpenclawV1Params): Promise<void> {
  await invoke('openclaw_send_v1', { params })
}

/** 检查 OpenClaw HTTP 服务是否在线（实际发起连接探测） */
export async function checkOpenclawAlive(baseUrl?: string): Promise<boolean> {
  return invoke<boolean>('check_openclaw_alive', { baseUrl: baseUrl || null })
}

/** 暂停/继续 AI 浏览器操控。暂停时，18790 端口所有浏览器操控接口返回 503。 */
export async function setAiPaused(paused: boolean): Promise<void> {
  await invoke('set_ai_paused', { paused })
}
