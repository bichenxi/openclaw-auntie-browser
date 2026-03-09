<script setup lang="ts">
import TabBar from '@/components/TabBar.vue'
import OpenclawPage from '@/components/OpenclawPage.vue'
import SettingsPage from '@/components/SettingsPage.vue'
import SkillsPage from '@/components/SkillsPage.vue'
import { useTabsStore } from '@/stores/tabs'
import { useProfileStore } from '@/stores/profile'
import { PROFILE_OPTIONS } from '@/stores/profile'

const store = useTabsStore()
const profileStore = useProfileStore()
const profileSwitching = ref(false)

onMounted(() => {
  profileStore.loadProfile()
})

async function selectProfile(name: string) {
  if (name === profileStore.currentProfile) return
  profileSwitching.value = true
  try {
    await profileStore.switchProfile(name)
  } finally {
    profileSwitching.value = false
  }
}

const profileLabels: Record<string, string> = {
  default: '默认',
  work: '工作',
  personal: '个人',
}
</script>

<template>
  <div class="flex flex-col h-screen overflow-hidden">
    <!-- 身份行 -->
    <div class="shrink-0 flex items-center gap-2.5 px-3 py-1.5 bg-[#faf8ff] border-b border-[#e8e2f4]">
      <span class="text-[12px] text-[#8a80a7]">身份</span>
      <div class="flex gap-1">
        <button
          v-for="name in PROFILE_OPTIONS"
          :key="name"
          type="button"
          class="px-2.5 py-1 text-[12px] bg-transparent border border-[#e8e2f4] rounded-[6px] cursor-pointer transition"
          :class="profileStore.currentProfile === name
            ? 'text-secondary border-secondary/40 bg-secondary/10'
            : 'text-[#8a80a7] hover:text-secondary hover:border-secondary/30 hover:bg-secondary/6'"
          :disabled="profileSwitching"
          @click="selectProfile(name)"
        >
          {{ profileLabels[name] ?? name }}
        </button>
      </div>
    </div>

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
