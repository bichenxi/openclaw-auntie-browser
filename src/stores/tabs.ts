import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'

const WEBVIEW_LOADING_DELAY_MS = 1200

export interface TabItem {
  id: string
  label: string
  url: string
  title: string
}

export const useTabsStore = defineStore('tabs', () => {
  const tabs = ref<TabItem[]>([])
  const activeTabId = ref<string | null>(null)
  const loadingTabId = ref<string | null>(null)
  let tabIndex = 0
  let loadingTimer: ReturnType<typeof setTimeout> | null = null

  const isHome = computed(() => activeTabId.value === null)
  const isWebviewLoading = computed(
    () => loadingTabId.value !== null && loadingTabId.value === activeTabId.value,
  )

  function scheduleShowWebview(label: string) {
    if (loadingTimer) clearTimeout(loadingTimer)
    loadingTabId.value = label
    loadingTimer = setTimeout(() => {
      loadingTimer = null
      loadingTabId.value = null
      invoke('show_webview', { label }).catch(() => {})
    }, WEBVIEW_LOADING_DELAY_MS)
  }

  async function openTab(url: string) {
    tabIndex++
    const id = `tab-${Date.now()}-${tabIndex}`
    const title = url.replace(/^https?:\/\//, '').split('/')[0]

    await invoke('create_tab_webview', { label: id, url })

    tabs.value.push({ id, label: id, url, title })

    if (activeTabId.value) {
      await invoke('hide_webview', { label: activeTabId.value }).catch(() => {})
    }
    activeTabId.value = id
    scheduleShowWebview(id)
  }

  async function switchTab(id: string) {
    if (id === activeTabId.value) return

    if (activeTabId.value) {
      await invoke('hide_webview', { label: activeTabId.value }).catch(() => {})
    }

    activeTabId.value = id
    await invoke('show_webview', { label: id }).catch(() => {})
  }

  async function switchToHome() {
    if (activeTabId.value) {
      await invoke('hide_webview', { label: activeTabId.value }).catch(() => {})
    }
    activeTabId.value = null
  }

  async function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return

    await invoke('close_webview', { label: id }).catch(() => {})
    tabs.value.splice(idx, 1)

    if (activeTabId.value === id) {
      if (tabs.value.length > 0) {
        const nextIdx = Math.min(idx, tabs.value.length - 1)
        await switchTab(tabs.value[nextIdx].id)
      } else {
        activeTabId.value = null
      }
    }
  }

  async function resizeAllWebviews() {
    const labels = tabs.value.map((t) => t.label)
    if (labels.length === 0) return
    await invoke('resize_all_webviews', { labels }).catch(() => {})
  }

  return {
    tabs,
    activeTabId,
    loadingTabId,
    isHome,
    isWebviewLoading,
    openTab,
    switchTab,
    switchToHome,
    closeTab,
    resizeAllWebviews,
  }
})
