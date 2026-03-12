<script setup lang="ts">
import { useTabsStore } from '@/stores/tabs'
import { useUrlInput } from '@/composables/useUrlInput'

const store = useTabsStore()
const { getTargetUrl } = useUrlInput()
const urlInput = ref('')
const isInputFocused = ref(false)

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
  <div class="relative w-full h-full bg-[linear-gradient(180deg,#f3eeff_0%,#f8f6ff_30%,#ffffff_70%,#fafafa_100%)] flex items-center justify-center overflow-hidden">
    <!-- 顶部光晕 -->
    <div class="absolute top-[-120px] left-1/2 -translate-x-1/2 w-[800px] h-[350px] bg-[radial-gradient(ellipse,rgba(95,71,206,0.12)_0%,rgba(95,71,206,0.04)_40%,transparent_70%)] pointer-events-none" />

    <!-- 中心内容 -->
    <div class="flex flex-col items-center gap-9 mt-[-40px]">
      <!-- 品牌 -->
      <div class="flex items-center gap-[14px]">
        <img
          class="w-14 h-14 rounded-[14px] object-cover shadow-[0_4px_20px_rgba(0,0,0,0.12)]"
          src="/logo.png"
          alt="logo"
        />
        <div class="flex flex-col">
          <span class="text-[28px] font-bold tracking-[-0.5px] text-secondary leading-[1.1]">Oclaw</span>
          <span class="text-[12px] text-[#9b8ec4] tracking-[3px] mt-0.5">你 的 专 属 浏 览 器</span>
        </div>
      </div>

      <!-- 搜索框 -->
      <div
        class="w-[620px] max-w-[90vw] flex items-center bg-white rounded-[28px] transition pr-1.5 pl-0 py-1"
        :class="isInputFocused
          ? 'shadow-[0_4px_28px_rgba(95,71,206,0.12),0_0_0_2px_rgba(95,71,206,0.15)]'
          : 'shadow-[0_2px_16px_rgba(95,71,206,0.06),0_0_0_1px_rgba(95,71,206,0.06)]'"
      >
        <input
          v-model="urlInput"
          type="text"
          placeholder="输入网址或关键词，回车打开 / 搜索"
          class="flex-1 border-none outline-none bg-transparent py-4 px-6 text-[15px] text-[#1f1f2e] placeholder-[#b8b0cc]"
          @keydown="handleKeydown"
          @focus="isInputFocused = true"
          @blur="isInputFocused = false"
        />
        <button
          class="w-11 h-11 rounded-full border-none bg-transparent text-[#9b8ec4] cursor-pointer flex items-center justify-center transition shrink-0 hover:text-secondary hover:bg-secondary/6 active:scale-[0.92]"
          @click="go"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
        </button>
      </div>

      <p class="text-[13px] text-[#b8b0cc] m-0 mt-[-12px]">
        或点击右上角 <strong class="text-[#9b8ec4]">OpenClaw</strong> 开始 AI 对话
      </p>
    </div>
  </div>
</template>
