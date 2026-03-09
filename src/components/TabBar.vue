<script setup lang="ts">
import { useTabsStore } from '@/stores/tabs'
import { getCurrentWindow } from '@tauri-apps/api/window'

const store = useTabsStore()

function startDrag(e: MouseEvent) {
  if (e.button !== 0) return
  getCurrentWindow().startDragging()
}
</script>

<template>
  <!-- tab-bar: -webkit-app-region:drag 无法原子化，保留在 style -->
  <div class="tab-bar flex items-stretch h-11 bg-[#f5f2fc] border-b border-[#e8e2f4] pr-2 gap-0.5 overflow-x-auto overflow-y-hidden shrink-0">
    <!-- 红绿灯占位拖拽区 -->
    <div class="drag-zone w-20 shrink-0" @mousedown="startDrag" />

    <!-- 首页 Tab -->
    <div
      class="tab-item flex items-center gap-1.5 px-3 min-w-auto rounded-t-lg cursor-pointer transition text-[#8a80a7] text-[13px] whitespace-nowrap relative mt-1.5"
      :class="store.isHome
        ? 'bg-white text-secondary shadow-[0_-1px_4px_rgba(95,71,206,0.06)]'
        : 'hover:bg-secondary/6 hover:text-secondary'"
      @click="store.switchToHome()"
    >
      <svg class="shrink-0" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
      <span class="overflow-hidden text-ellipsis whitespace-nowrap leading-none">主页</span>
    </div>

    <!-- 网页 Tabs -->
    <div
      v-for="tab in store.tabs"
      :key="tab.id"
      class="tab-item flex items-center gap-1.5 px-3.5 min-w-[60px] max-w-[200px] rounded-t-lg cursor-pointer transition text-[#8a80a7] text-[13px] whitespace-nowrap relative mt-1.5"
      :class="store.activeTabId === tab.id && store.specialView === null
        ? 'bg-white text-secondary shadow-[0_-1px_4px_rgba(95,71,206,0.06)]'
        : 'hover:bg-secondary/6 hover:text-secondary'"
      @click="store.switchTab(tab.id)"
    >
      <span class="overflow-hidden text-ellipsis whitespace-nowrap leading-none">{{ tab.title }}</span>
      <button
        class="flex items-center justify-center w-[18px] h-[18px] rounded-[4px] border-none bg-transparent text-[#b8b0cc] cursor-pointer shrink-0 transition p-0 hover:bg-[rgba(239,68,68,0.12)] hover:text-accent"
        @click.stop="store.closeTab(tab.id)"
      >
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>

    <!-- 占位（可拖拽区域） -->
    <div class="tab-spacer flex-1 min-w-[40px]" @mousedown="startDrag" />

    <!-- OpenClaw 按钮 -->
    <div
      class="tab-item flex items-center gap-[5px] px-3 min-w-auto rounded-t-lg cursor-pointer transition text-[#8a80a7] text-[13px] whitespace-nowrap relative mt-1.5"
      :class="store.specialView === 'openclaw'
        ? 'bg-[rgba(124,92,252,0.12)] text-[#7c5cfc]'
        : 'hover:bg-secondary/6 hover:text-secondary'"
      @click="store.switchToSpecialView('openclaw')"
    >
      <img class="w-[18px] h-[18px] rounded-[5px] object-cover shrink-0" src="/logo.jpg" alt="logo" />
      <span class="overflow-hidden text-ellipsis whitespace-nowrap leading-none">OpenClaw</span>
    </div>

    <!-- 技能管理按钮 -->
    <div
      class="tab-item flex items-center gap-[5px] px-3 min-w-auto rounded-t-lg cursor-pointer transition text-[#8a80a7] text-[13px] whitespace-nowrap relative mt-1.5"
      :class="store.specialView === 'skills'
        ? 'bg-secondary/10 text-secondary'
        : 'hover:bg-secondary/6 hover:text-secondary'"
      @click="store.switchToSpecialView('skills')"
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" />
        <polyline points="10 9 9 9 8 9" />
      </svg>
      <span class="overflow-hidden text-ellipsis whitespace-nowrap leading-none">技能</span>
    </div>

    <!-- 设置按钮 -->
    <div
      class="tab-item flex items-center gap-[5px] px-3 min-w-auto rounded-t-lg cursor-pointer transition text-[#8a80a7] text-[13px] whitespace-nowrap relative mt-1.5"
      :class="store.specialView === 'settings'
        ? 'bg-secondary/10 text-secondary'
        : 'hover:bg-secondary/6 hover:text-secondary'"
      @click="store.switchToSpecialView('settings')"
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
      <span class="overflow-hidden text-ellipsis whitespace-nowrap leading-none">设置</span>
    </div>
  </div>
</template>

<style scoped>
/* -webkit-app-region 无法用 UnoCSS 原子类表达，保留此处 */
.tab-bar {
  -webkit-app-region: drag;
}

.tab-bar::-webkit-scrollbar {
  display: none;
}

.tab-item {
  -webkit-app-region: no-drag;
}

.drag-zone,
.tab-spacer {
  -webkit-app-region: drag;
  cursor: default;
}
</style>
