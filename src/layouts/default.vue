<script setup lang="ts">
import TabBar from '@/components/TabBar.vue'
import { useTabsStore } from '@/stores/tabs'

const store = useTabsStore()
</script>

<template>
  <div class="layout">
    <TabBar />
    <div class="layout-content">
      <RouterView v-show="store.isHome" />
      <!-- webview 加载期间在内容区显示动画（webview 创建时先隐藏，延迟后再 show） -->
      <Transition name="fade">
        <div v-if="!store.isHome && store.isWebviewLoading" class="webview-loading">
          <div class="webview-loading-spinner" />
          <span class="webview-loading-text">加载中...</span>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.layout-content {
  flex: 1;
  min-height: 0;
  position: relative;
}

.webview-loading {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, #f8f6ff 0%, #f3eeff 100%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}

.webview-loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(95, 71, 206, 0.15);
  border-top-color: #5f47ce;
  border-radius: 50%;
  animation: webview-spin 0.85s linear infinite;
}

.webview-loading-text {
  font-size: 13px;
  color: #9b8ec4;
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
