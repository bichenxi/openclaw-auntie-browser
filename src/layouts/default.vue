<script setup lang="ts">
import TabBar from '@/components/TabBar.vue'
import OpenclawPage from '@/components/OpenclawPage.vue'
import SettingsPage from '@/components/SettingsPage.vue'
import SkillsPage from '@/components/SkillsPage.vue'
import { useTabsStore } from '@/stores/tabs'

const store = useTabsStore()
</script>

<template>
  <div class="flex flex-col h-screen overflow-hidden">
    <TabBar />

    <!-- 内容区 -->
    <div class="flex-1 min-h-0 overflow-hidden relative">
      <OpenclawPage v-if="store.specialView === 'openclaw'" />
      <SettingsPage v-else-if="store.specialView === 'settings'" />
      <SkillsPage v-else-if="store.specialView === 'skills'" />
      <RouterView v-else-if="store.isHome" />
      <Transition v-else name="fade">
        <div
          v-if="store.isWebviewLoading"
          class="absolute inset-0 bg-[linear-gradient(180deg,#f8f6ff_0%,#f3eeff_100%)] flex flex-col items-center justify-center gap-4"
        >
          <div class="webview-loading-spinner" />
          <span class="text-[13px] text-[#9b8ec4]">加载中...</span>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.webview-loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(95, 71, 206, 0.15);
  border-top-color: #5f47ce;
  border-radius: 50%;
  animation: webview-spin 0.85s linear infinite;
}

@keyframes webview-spin {
  to {
    transform: rotate(360deg);
  }
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
