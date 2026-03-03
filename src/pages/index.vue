<script setup lang="ts">
import { useTabsStore } from '@/stores/tabs'

const store = useTabsStore()
const urlInput = ref('')
const isInputFocused = ref(false)

const SEARCH_ENGINE_URL = 'https://www.google.com/search'

function isLikelyUrl(raw: string): boolean {
  const s = raw.trim()
  if (!s || /\s/.test(s)) return false
  if (/^https?:\/\//i.test(s)) return true
  // 形如 xxx.yy（含常见 TLD）视为网址
  if (/^.+\.[a-z]{2,6}(\/.*)?$/i.test(s)) return true
  return false
}

function normalizeUrl(raw: string): string {
  const s = raw.trim()
  if (!s) return ''
  if (/^https?:\/\//i.test(s)) return s
  return `https://${s}`
}

function getTargetUrl(raw: string): string {
  const s = raw.trim()
  if (!s) return ''
  if (isLikelyUrl(s)) return normalizeUrl(s)
  const q = encodeURIComponent(s)
  return `${SEARCH_ENGINE_URL}?q=${q}`
}

async function go() {
  const url = getTargetUrl(urlInput.value)
  if (!url) return
  await store.openTab(url)
  urlInput.value = ''
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') go()
}
</script>

<template>
  <div class="home-page">
    <div class="top-glow" />

    <div class="center-content">
      <div class="brand">
        <div class="brand-icon">
          <div class="brand-diamond" />
        </div>
        <div class="brand-text">
          <span class="brand-name">OpenClaw</span>
          <span class="brand-sub">你 的 专 属 浏 览 器</span>
        </div>
      </div>

      <div class="search-bar" :class="{ focused: isInputFocused }">
        <input
          v-model="urlInput"
          type="text"
          placeholder="输入网址或关键词，回车打开 / 搜索"
          @keydown="handleKeydown"
          @focus="isInputFocused = true"
          @blur="isInputFocused = false"
        />
        <button class="search-btn" @click="go">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home-page {
  position: relative;
  width: 100%;
  height: 100%;
  background: linear-gradient(180deg, #f3eeff 0%, #f8f6ff 30%, #ffffff 70%, #fafafa 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.top-glow {
  position: absolute;
  top: -120px;
  left: 50%;
  transform: translateX(-50%);
  width: 800px;
  height: 350px;
  background: radial-gradient(ellipse, rgba(95, 71, 206, 0.12) 0%, rgba(95, 71, 206, 0.04) 40%, transparent 70%);
  pointer-events: none;
}

.center-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 36px;
  margin-top: -40px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 14px;
}

.brand-icon {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  background: linear-gradient(135deg, #7c5cfc 0%, #5f47ce 100%);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 20px rgba(95, 71, 206, 0.3);
}

.brand-diamond {
  width: 22px;
  height: 22px;
  border: 3px solid rgba(255, 255, 255, 0.9);
  border-radius: 5px;
  transform: rotate(45deg);
}

.brand-text {
  display: flex;
  flex-direction: column;
}

.brand-name {
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.5px;
  color: #5f47ce;
  line-height: 1.1;
}

.brand-sub {
  font-size: 12px;
  color: #9b8ec4;
  letter-spacing: 3px;
  margin-top: 2px;
}

.search-bar {
  width: 620px;
  max-width: 90vw;
  display: flex;
  align-items: center;
  background: #ffffff;
  border-radius: 28px;
  box-shadow: 0 2px 16px rgba(95, 71, 206, 0.06), 0 0 0 1px rgba(95, 71, 206, 0.06);
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  padding: 4px 6px 4px 0;
}

.search-bar.focused {
  box-shadow: 0 4px 28px rgba(95, 71, 206, 0.12), 0 0 0 2px rgba(95, 71, 206, 0.15);
}

.search-bar input {
  flex: 1;
  border: none;
  outline: none;
  background: none;
  padding: 16px 24px;
  font-size: 15px;
  color: #1f1f2e;
}

.search-bar input::placeholder {
  color: #b8b0cc;
}

.search-btn {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: none;
  background: none;
  color: #9b8ec4;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  flex-shrink: 0;
}

.search-btn:hover {
  color: #5f47ce;
  background: rgba(95, 71, 206, 0.06);
}

.search-btn:active {
  transform: scale(0.92);
}
</style>
