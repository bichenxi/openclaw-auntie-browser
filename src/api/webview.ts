import { invoke } from '@tauri-apps/api/core'

export async function createTabWebview(label: string, url: string, rightMargin: number): Promise<void> {
  await invoke('create_tab_webview', { label, url, rightMargin })
}

export async function showWebview(label: string): Promise<void> {
  await invoke('show_webview', { label })
}

export async function hideWebview(label: string): Promise<void> {
  await invoke('hide_webview', { label })
}

export async function closeWebview(label: string): Promise<void> {
  await invoke('close_webview', { label })
}

export async function resizeAllWebviews(labels: string[], rightMargin: number): Promise<void> {
  await invoke('resize_all_webviews', { labels, rightMargin })
}

export async function evalInWebview(label: string, script: string): Promise<void> {
  await invoke('eval_in_webview', { label, script })
}

export async function getDomSnapshot(label: string): Promise<void> {
  await invoke('get_dom_snapshot', { label })
}

/** 同步当前活动 tab 给 Rust，供 18790 HTTP API 使用；null 表示在首页 */
export async function setActiveTabLabel(label: string | null): Promise<void> {
  await invoke('set_active_tab_label', { label })
}
