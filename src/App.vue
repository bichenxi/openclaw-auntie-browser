<script setup lang="ts">
import { listen } from '@tauri-apps/api/event'
import { useTabsStore } from '@/stores/tabs'
import { useInstallerStore } from '@/stores/installer'
import { checkOpenclawAlive } from '@/api/openclaw'
import { checkOpenclawInstalled } from '@/api/installer'

const store = useTabsStore()
const installerStore = useInstallerStore()

const onResize = useDebounceFn(() => {
  store.resizeAllWebviews()
}, 100)

let unlistenApiOpenTab: (() => void) | null = null

let alivePoller: ReturnType<typeof setInterval> | null = null

async function detectAndRedirect() {
  // 安装进行中不检测，避免干扰
  if (installerStore.installing) return
  const alive = await checkOpenclawAlive().catch(() => false)
  if (!alive && store.specialView !== 'setup') {
    const status = await checkOpenclawInstalled().catch(() => ({ npm_installed: false, onboarded: false }))
    installerStore.isInstalled = status.npm_installed || status.onboarded
    installerStore.isOnboarded = status.onboarded
    store.switchToSpecialView('setup')
  }
}

onMounted(async () => {
  window.addEventListener('resize', onResize)
  listen<{ url: string }>('api_open_tab', (e) => {
    store.openTab(e.payload.url)
  }).then((fn) => {
    unlistenApiOpenTab = fn
  }).catch(() => {})

  // 启动时立即检测
  await detectAndRedirect()

  // 每 8 秒持续检测，离线时立刻跳转
  alivePoller = setInterval(detectAndRedirect, 8000)
})

onUnmounted(() => {
  window.removeEventListener('resize', onResize)
  unlistenApiOpenTab?.()
  if (alivePoller) clearInterval(alivePoller)
})
</script>

<template>
  <RouterView />
</template>
