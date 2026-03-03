<script setup lang="ts">
import { useTabsStore } from '@/stores/tabs'

const store = useTabsStore()
</script>

<template>
  <div class="tab-bar">
    <!-- 首页 Tab -->
    <div
      class="tab-item home-tab"
      :class="{ active: store.isHome }"
      @click="store.switchToHome()"
    >
      <svg class="tab-home-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
      <span class="tab-title">主页</span>
    </div>

    <!-- 网页 Tabs -->
    <div
      v-for="tab in store.tabs"
      :key="tab.id"
      class="tab-item"
      :class="{ active: store.activeTabId === tab.id }"
      @click="store.switchTab(tab.id)"
    >
      <span class="tab-title">{{ tab.title }}</span>
      <button
        class="tab-close"
        @click.stop="store.closeTab(tab.id)"
      >
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>

    <!-- 占位 -->
    <div class="tab-spacer" />
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  align-items: stretch;
  height: 44px;
  background: #f5f2fc;
  border-bottom: 1px solid #e8e2f4;
  padding: 0 8px;
  gap: 2px;
  overflow-x: auto;
  overflow-y: hidden;
  flex-shrink: 0;
  -webkit-app-region: drag;
}

.tab-bar::-webkit-scrollbar {
  display: none;
}

.tab-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 14px;
  min-width: 60px;
  max-width: 200px;
  border-radius: 8px 8px 0 0;
  cursor: pointer;
  transition: all 0.15s;
  color: #8a80a7;
  font-size: 13px;
  white-space: nowrap;
  position: relative;
  -webkit-app-region: no-drag;
  margin-top: 6px;
}

.tab-item:hover {
  background: rgba(95, 71, 206, 0.06);
  color: #5f47ce;
}

.tab-item.active {
  background: #ffffff;
  color: #5f47ce;
  box-shadow: 0 -1px 4px rgba(95, 71, 206, 0.06);
}

.home-tab {
  min-width: auto;
  padding: 0 12px;
}

.tab-home-icon {
  flex-shrink: 0;
}

.tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1;
}

.tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: none;
  background: none;
  color: #b8b0cc;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
  padding: 0;
}

.tab-close:hover {
  background: rgba(239, 68, 68, 0.12);
  color: #ef4444;
}

.tab-spacer {
  flex: 1;
  min-width: 40px;
  -webkit-app-region: drag;
}
</style>
