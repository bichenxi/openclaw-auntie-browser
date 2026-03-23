<script setup lang="ts">
import { useOpenclawStore } from '@/stores/openclaw'
import type { FlowNodeState } from '@/stores/openclaw'

const props = defineProps<{ executionId: string }>()
const ocStore = useOpenclawStore()

const exec = computed(() => ocStore.flowExecutions[props.executionId])

function nodesByLevel(ids: string[]): FlowNodeState[] {
  return ids.map(id => exec.value?.nodes.find(n => n.id === id)).filter(Boolean) as FlowNodeState[]
}

const overallIcon = computed(() => {
  const s = exec.value?.status
  if (s === 'completed') return 'ok'
  if (s === 'failed') return 'err'
  return 'running'
})

const elapsed = ref(0)
let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => { timer = setInterval(() => elapsed.value++, 1000) })
onUnmounted(() => { if (timer) clearInterval(timer) })
watch(() => exec.value?.status, (s) => { if (s !== 'running' && timer) { clearInterval(timer); timer = null } })
</script>

<template>
  <div v-if="exec" class="w-full max-w-[520px] rounded-[14px] border border-[#e8e2f4] bg-white shadow-[0_2px_12px_rgba(95,71,206,0.07)] overflow-hidden">

    <!-- 卡片头 -->
    <div class="flex items-start gap-3 px-4 py-3 border-b border-[#f0ecfa] bg-[#faf9ff]">
      <div class="w-8 h-8 rounded-[9px] bg-[linear-gradient(135deg,#7c5cfc,#5f47ce)] flex-center shrink-0 mt-0.5">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="5" cy="12" r="2"/><circle cx="19" cy="5" r="2"/><circle cx="19" cy="19" r="2"/>
          <line x1="7" y1="12" x2="17" y2="6"/><line x1="7" y1="12" x2="17" y2="18"/>
        </svg>
      </div>
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span class="text-[13px] font-semibold text-[#1f1f2e] truncate">{{ exec.flowName }}</span>
          <!-- 状态徽章 -->
          <span
            class="shrink-0 flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold"
            :class="{
              'bg-blue-50 text-blue-600 border border-blue-200': overallIcon === 'running',
              'bg-emerald-50 text-emerald-600 border border-emerald-200': overallIcon === 'ok',
              'bg-red-50 text-red-500 border border-red-200': overallIcon === 'err',
            }"
          >
            <svg v-if="overallIcon === 'running'" class="animate-spin" width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M21 12a9 9 0 1 1-6.2-8.6"/></svg>
            <svg v-else-if="overallIcon === 'ok'" width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            <svg v-else width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            {{ overallIcon === 'running' ? `执行中 ${elapsed}s` : overallIcon === 'ok' ? '已完成' : '执行失败' }}
          </span>
        </div>
        <div class="text-[11px] text-[#9b8ec4] mt-0.5 truncate">{{ exec.task }}</div>
      </div>
    </div>

    <!-- 层级节点 -->
    <div class="flex flex-col divide-y divide-[#f5f3ff]">
      <div v-for="(levelIds, li) in exec.levelIds" :key="li" class="px-4 py-2.5">

        <!-- 并行标签 -->
        <div v-if="levelIds.length > 1" class="flex items-center gap-1 mb-2">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#d97706" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
          <span class="text-[10px] font-semibold text-amber-600 uppercase tracking-wider">并行</span>
        </div>

        <!-- 节点行 -->
        <div :class="levelIds.length > 1 ? 'grid gap-2' : ''" :style="levelIds.length > 1 ? `grid-template-columns: repeat(${levelIds.length}, 1fr)` : ''">
          <div
            v-for="node in nodesByLevel(levelIds)"
            :key="node.id"
            class="flex flex-col gap-1 p-2.5 rounded-[10px] border transition-all duration-300"
            :class="{
              'bg-[#f8f8fb] border-[#ede8f8]': node.status === 'pending',
              'bg-blue-50 border-blue-200': node.status === 'running',
              'bg-emerald-50 border-emerald-200': node.status === 'completed',
              'bg-red-50 border-red-200': node.status === 'failed',
            }"
          >
            <!-- 节点头：图标 + 名称 -->
            <div class="flex items-center gap-1.5">
              <!-- 状态图标 -->
              <span class="shrink-0 w-4 h-4 flex-center">
                <svg v-if="node.status === 'running'" class="animate-spin" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#3b82f6" stroke-width="3"><path d="M21 12a9 9 0 1 1-6.2-8.6"/></svg>
                <svg v-else-if="node.status === 'completed'" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                <svg v-else-if="node.status === 'failed'" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                <svg v-else width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#c4bdd8" stroke-width="2"><circle cx="12" cy="12" r="9"/></svg>
              </span>
              <span
                class="text-[12px] font-semibold truncate"
                :class="{
                  'text-[#b8b0cc]': node.status === 'pending',
                  'text-blue-700': node.status === 'running',
                  'text-emerald-700': node.status === 'completed',
                  'text-red-600': node.status === 'failed',
                }"
              >{{ node.label }}</span>
            </div>
            <!-- 输出/状态文本 -->
            <div class="text-[11px] leading-[1.5] pl-[22px]">
              <span v-if="node.status === 'pending'" class="text-[#c4bdd8]">等待中...</span>
              <span v-else-if="node.status === 'running'" class="text-blue-400 flex items-center gap-1">
                <span class="inline-flex gap-0.5">
                  <span class="w-[4px] h-[4px] rounded-full bg-blue-400 animate-[typing-dot_1.2s_ease-in-out_infinite]"/>
                  <span class="w-[4px] h-[4px] rounded-full bg-blue-400 animate-[typing-dot_1.2s_ease-in-out_infinite_0.2s]"/>
                  <span class="w-[4px] h-[4px] rounded-full bg-blue-400 animate-[typing-dot_1.2s_ease-in-out_infinite_0.4s]"/>
                </span>
                思考中
              </span>
              <span v-else-if="node.status === 'failed'" class="text-red-500 line-clamp-2">{{ node.error }}</span>
              <span v-else class="text-[#4b4568] line-clamp-2 break-words">{{ node.output }}</span>
            </div>
          </div>
        </div>

      </div>
    </div>

  </div>
</template>

<style scoped>
@keyframes typing-dot {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1); }
}
</style>
