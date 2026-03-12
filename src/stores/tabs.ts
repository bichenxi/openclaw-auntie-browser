import { defineStore } from 'pinia'
import * as webviewApi from '@/api/webview'
import { setAiPaused as apiSetAiPaused } from '@/api/openclaw'
import { useRecordingStore } from '@/stores/recording'

const WEBVIEW_LOADING_DELAY_MS = 1200
const CHAT_SIDEBAR_WIDTH = 300
const SIDEBAR_HANDLE_WIDTH = 36

export interface TabItem {
  id: string
  label: string
  url: string
  title: string
}

export type SpecialView = 'openclaw' | 'settings' | 'skills' | 'setup'

export const useTabsStore = defineStore('tabs', () => {
  const tabs = ref<TabItem[]>([])
  const activeTabId = ref<string | null>(null)
  const loadingTabId = ref<string | null>(null)
  const specialView = ref<SpecialView | null>('openclaw')
  const sidebarOpen = ref(false)
  let tabIndex = 0
  let loadingTimer: ReturnType<typeof setTimeout> | null = null

  const isHome = computed(() => activeTabId.value === null && specialView.value === null)
  const isWebviewLoading = computed(
    () => loadingTabId.value !== null && loadingTabId.value === activeTabId.value,
  )

  // 人机混合接管：true 表示已暂停 AI，用户可手动操作右侧网页，完成后「继续 AI」
  const aiPaused = ref(false)
  async function setAiPaused(value: boolean) {
    await apiSetAiPaused(value).catch(() => {})
    aiPaused.value = value
  }

  async function toggleSidebar() {
    sidebarOpen.value = !sidebarOpen.value
    await resizeAllWebviews()
  }

  async function switchToSpecialView(view: SpecialView) {
    if (specialView.value === view) return
    if (activeTabId.value) {
      await webviewApi.hideWebview(activeTabId.value).catch(() => {})
    }
    specialView.value = view
    webviewApi.setActiveTabLabel(null).catch(() => {})
  }

  function scheduleShowWebview(label: string) {
    if (loadingTimer) clearTimeout(loadingTimer)
    loadingTabId.value = label
    loadingTimer = setTimeout(() => {
      loadingTimer = null
      loadingTabId.value = null
      webviewApi.showWebview(label).catch(() => {})
    }, WEBVIEW_LOADING_DELAY_MS)
  }

  async function openTab(url: string) {
    specialView.value = null
    tabIndex++
    const id = `tab-${Date.now()}-${tabIndex}`
    const title = url.replace(/^https?:\/\//, '').split('/')[0]

    await webviewApi.createTabWebview(id, url, SIDEBAR_HANDLE_WIDTH + (sidebarOpen.value ? CHAT_SIDEBAR_WIDTH : 0))
    useRecordingStore().pushStep({ type: 'navigate', url })

    tabs.value.push({ id, label: id, url, title })

    if (activeTabId.value) {
      await webviewApi.hideWebview(activeTabId.value).catch(() => {})
    }
    activeTabId.value = id
    scheduleShowWebview(id)
    webviewApi.setActiveTabLabel(id).catch(() => {})
  }

  async function switchTab(id: string) {
    if (id === activeTabId.value && specialView.value === null) return
    specialView.value = null

    if (activeTabId.value) {
      await webviewApi.hideWebview(activeTabId.value).catch(() => {})
    }

    activeTabId.value = id
    await webviewApi.showWebview(id).catch(() => {})
    webviewApi.setActiveTabLabel(id).catch(() => {})
  }

  async function switchToHome() {
    specialView.value = null
    if (activeTabId.value) {
      await webviewApi.hideWebview(activeTabId.value).catch(() => {})
    }
    activeTabId.value = null
    webviewApi.setActiveTabLabel(null).catch(() => {})
  }

  async function closeTab(id: string) {
    const idx = tabs.value.findIndex((t) => t.id === id)
    if (idx === -1) return

    await webviewApi.closeWebview(id).catch(() => {})
    tabs.value.splice(idx, 1)

    if (activeTabId.value === id) {
      if (tabs.value.length > 0) {
        const nextIdx = Math.min(idx, tabs.value.length - 1)
        await switchTab(tabs.value[nextIdx].id)
      } else {
        activeTabId.value = null
        webviewApi.setActiveTabLabel(null).catch(() => {})
      }
    }
  }

  async function resizeAllWebviews() {
    const labels = tabs.value.map((t) => t.label)
    if (labels.length === 0) return
    await webviewApi.resizeAllWebviews(labels, SIDEBAR_HANDLE_WIDTH + (sidebarOpen.value ? CHAT_SIDEBAR_WIDTH : 0)).catch(() => {})
  }

  return {
    tabs,
    activeTabId,
    loadingTabId,
    specialView,
    sidebarOpen,
    isHome,
    isWebviewLoading,
    aiPaused,
    setAiPaused,
    toggleSidebar,
    switchToSpecialView,
    openTab,
    switchTab,
    switchToHome,
    closeTab,
    resizeAllWebviews,
  }
})
