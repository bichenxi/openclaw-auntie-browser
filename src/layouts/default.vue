<script setup lang="ts">
import TabBar from '@/components/TabBar.vue'
import OpenclawPage from '@/views/OpenclawPage.vue'
import SettingsPage from '@/views/SettingsPage.vue'
import SkillsPage from '@/views/SkillsPage.vue'
import SetupPage from '@/views/SetupPage.vue'
import { useTabsStore } from '@/stores/tabs'

const store = useTabsStore()
</script>

<template>
  <div class="flex flex-col h-screen overflow-hidden">
    <TabBar />

    <!-- 内容区 -->
    <div class="flex-1 min-h-0 overflow-hidden relative">
      <SetupPage v-if="store.specialView === 'setup'" />
      <OpenclawPage v-else-if="store.specialView === 'openclaw'" />
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

      <!-- 聊天侧边栏：webview 活动时悬浮在右侧 -->
      <template v-if="store.activeTabId !== null && store.specialView === null">
        <!-- 侧边栏面板 -->
        <div
          v-if="store.sidebarOpen"
          class="absolute right-9 top-0 bottom-0 w-[300px] border-l border-[#e8e2f4] shadow-[-4px_0_16px_rgba(95,71,206,0.08)] bg-white z-40 overflow-hidden"
        >
          <OpenclawPage />
        </div>

        <!-- 右侧固定拨片（36px，始终可见，点击展开/收起） -->
        <div
          class="absolute right-0 top-0 bottom-0 w-9 flex flex-col items-center justify-center gap-2 cursor-pointer z-50 border-l select-none"
          :class="store.sidebarOpen
            ? 'bg-[rgba(124,92,252,0.06)] border-[#d4cdf4]'
            : 'bg-[rgba(245,242,252,0.92)] border-[#e8e2f4] hover:bg-[rgba(124,92,252,0.06)]'"
          @click="store.toggleSidebar()"
        >
          <img src="/logo.jpg" class="w-5 h-5 rounded-[5px] object-cover shadow-sm shrink-0" alt="AI" />
          <svg
            class="transition-transform duration-200 text-[#9b8ec4]"
            :class="store.sidebarOpen ? '' : 'rotate-180'"
            width="10" height="10" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"
          >
            <polyline points="9 18 15 12 9 6" />
          </svg>
        </div>
      </template>
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
