<script setup lang="ts">
import { useOpenclawStore } from '@/stores/openclaw'
import type { FlowNodeState } from '@/stores/openclaw'

const props = defineProps<{ executionId: string }>()
const ocStore = useOpenclawStore()
const exec = computed(() => ocStore.flowExecutions[props.executionId])
const hasBranches = computed(() => (exec.value?.branches?.length ?? 0) > 1)

const elapsed = ref(0)
let timer: ReturnType<typeof setInterval> | null = null
onMounted(() => { timer = setInterval(() => elapsed.value++, 1000) })
onUnmounted(() => { if (timer) clearInterval(timer) })
watch(() => exec.value?.status, s => { if (s !== 'running' && timer) { clearInterval(timer); timer = null } })

const selectedNode = ref<FlowNodeState | null>(null)
const copied = ref(false)

function nodeById(id: string): FlowNodeState | undefined {
  return exec.value?.nodes.find(n => n.id === id)
}
function openDetail(id: string) {
  const n = nodeById(id)
  if (n) selectedNode.value = n
}
async function copyOutput() {
  if (!selectedNode.value?.output) return
  try { await navigator.clipboard.writeText(selectedNode.value.output) } catch {}
  copied.value = true
  setTimeout(() => copied.value = false, 1500)
}

function statusColor(status: FlowNodeState['status'] | string) {
  if (status === 'running') return 'text-cyan-400'
  if (status === 'completed') return 'text-emerald-400'
  if (status === 'failed') return 'text-red-400'
  return 'text-[#3d365c]'
}
</script>

<template>
  <div v-if="exec" class="w-full max-w-[560px] rounded-[12px] overflow-hidden border border-[#2a2347] shadow-[0_4px_32px_rgba(0,0,0,0.5)]">

    <!-- Title bar (macOS style) -->
    <div class="flex items-center gap-2 px-4 py-2.5 bg-[#17152b] border-b border-[#2a2347]">
      <div class="flex gap-1.5 shrink-0">
        <div class="w-2.5 h-2.5 rounded-full bg-[#ff5f57]"/>
        <div class="w-2.5 h-2.5 rounded-full bg-[#febc2e]"/>
        <div class="w-2.5 h-2.5 rounded-full bg-[#28c840]"/>
      </div>
      <span class="flex-1 text-center text-[11px] font-mono text-[#6b5aad] truncate">{{ exec.flowName }}</span>
      <span
        class="shrink-0 text-[10px] font-mono"
        :class="{
          'text-cyan-400': exec.status === 'running',
          'text-emerald-400': exec.status === 'completed',
          'text-red-400': exec.status === 'failed',
        }"
      >{{ exec.status === 'running' ? `running · ${elapsed}s` : exec.status === 'completed' ? 'done' : 'failed' }}</span>
    </div>

    <!-- Terminal body -->
    <div class="bg-[#0d0c1a] px-4 pt-3 pb-4 flex flex-col gap-2">

      <!-- Task line -->
      <div class="flex items-start gap-1.5 mb-1">
        <span class="font-mono text-[10px] text-[#4a3e7a] shrink-0 mt-[1px]">&gt;</span>
        <span class="font-mono text-[11px] text-[#4a3e7a] leading-relaxed">{{ exec.task }}</span>
      </div>

      <!-- Branch columns (parallel) -->
      <div
        v-if="hasBranches"
        class="grid gap-2 items-start"
        :style="`grid-template-columns: repeat(${exec.branches.length}, 1fr)`"
      >
        <div v-for="(branch, bi) in exec.branches" :key="bi" class="flex flex-col gap-1.5">
          <template v-for="nodeId in branch" :key="nodeId">
            <div
              v-if="nodeById(nodeId)"
              class="flex flex-col gap-1 px-2.5 py-2 rounded-[7px] border border-[#1e1b35] cursor-pointer transition-colors hover:border-[#3a3264] hover:bg-[#13112a]"
              :class="{ 'border-[#3a3264] bg-[#13112a]': selectedNode?.id === nodeId }"
              @click="openDetail(nodeId)"
            >
              <div class="flex items-center justify-between gap-1">
                <div class="flex items-center gap-1">
                  <span class="font-mono text-[10px] text-[#3d365c]">$</span>
                  <span class="font-mono text-[11px] font-semibold truncate" :class="statusColor(nodeById(nodeId)!.status)">{{ nodeById(nodeId)!.label }}</span>
                </div>
                <span class="font-mono text-[9px] uppercase tracking-wider shrink-0" :class="statusColor(nodeById(nodeId)!.status)">
                  {{ nodeById(nodeId)!.status === 'pending' ? 'idle' : nodeById(nodeId)!.status }}
                </span>
              </div>
              <div class="pl-3 font-mono text-[10px] leading-relaxed min-h-[14px]">
                <span v-if="nodeById(nodeId)!.status === 'pending'" class="text-[#2e2852]">_</span>
                <span v-else-if="nodeById(nodeId)!.status === 'failed'" class="text-red-500 line-clamp-2 break-all">{{ nodeById(nodeId)!.error ?? 'error' }}</span>
                <span v-else class="text-[#5c4e94] line-clamp-2 break-words">
                  {{ nodeById(nodeId)!.output }}<span v-if="nodeById(nodeId)!.status === 'running'" class="cursor-blink text-cyan-400">█</span>
                </span>
              </div>
            </div>
          </template>
        </div>
      </div>

      <!-- Single branch (sequential) -->
      <template v-else-if="exec.branches.length === 1">
        <template v-for="nodeId in exec.branches[0]" :key="nodeId">
          <div
            v-if="nodeById(nodeId)"
            class="flex flex-col gap-1 px-2.5 py-2 rounded-[7px] border border-[#1e1b35] cursor-pointer transition-colors hover:border-[#3a3264] hover:bg-[#13112a]"
            :class="{ 'border-[#3a3264] bg-[#13112a]': selectedNode?.id === nodeId }"
            @click="openDetail(nodeId)"
          >
            <div class="flex items-center justify-between gap-1">
              <div class="flex items-center gap-1">
                <span class="font-mono text-[10px] text-[#3d365c]">$</span>
                <span class="font-mono text-[11px] font-semibold truncate" :class="statusColor(nodeById(nodeId)!.status)">{{ nodeById(nodeId)!.label }}</span>
              </div>
              <span class="font-mono text-[9px] uppercase tracking-wider shrink-0" :class="statusColor(nodeById(nodeId)!.status)">
                {{ nodeById(nodeId)!.status === 'pending' ? 'idle' : nodeById(nodeId)!.status }}
              </span>
            </div>
            <div class="pl-3 font-mono text-[10px] leading-relaxed min-h-[14px]">
              <span v-if="nodeById(nodeId)!.status === 'pending'" class="text-[#2e2852]">_</span>
              <span v-else-if="nodeById(nodeId)!.status === 'failed'" class="text-red-500 line-clamp-2 break-all">{{ nodeById(nodeId)!.error ?? 'error' }}</span>
              <span v-else class="text-[#5c4e94] line-clamp-2 break-words">
                {{ nodeById(nodeId)!.output }}<span v-if="nodeById(nodeId)!.status === 'running'" class="cursor-blink text-cyan-400">█</span>
              </span>
            </div>
          </div>
        </template>
      </template>

      <!-- Convergence separator -->
      <div v-if="hasBranches && exec.convergeIds.length > 0" class="flex items-center gap-2 my-0.5">
        <div class="flex-1 border-t border-dashed border-[#2a2347]"/>
        <span class="font-mono text-[8px] uppercase tracking-[0.15em] text-[#3d365c]">converge</span>
        <div class="flex-1 border-t border-dashed border-[#2a2347]"/>
      </div>

      <!-- Convergence nodes -->
      <template v-for="nodeId in exec.convergeIds" :key="nodeId">
        <div
          v-if="nodeById(nodeId)"
          class="flex flex-col gap-1 px-2.5 py-2 rounded-[7px] border border-[#1e1b35] cursor-pointer transition-colors hover:border-[#3a3264] hover:bg-[#13112a]"
          :class="{ 'border-[#3a3264] bg-[#13112a]': selectedNode?.id === nodeId }"
          @click="openDetail(nodeId)"
        >
          <div class="flex items-center justify-between gap-1">
            <div class="flex items-center gap-1">
              <span class="font-mono text-[10px] text-[#3d365c]">$</span>
              <span class="font-mono text-[11px] font-semibold truncate" :class="statusColor(nodeById(nodeId)!.status)">{{ nodeById(nodeId)!.label }}</span>
            </div>
            <span class="font-mono text-[9px] uppercase tracking-wider shrink-0" :class="statusColor(nodeById(nodeId)!.status)">
              {{ nodeById(nodeId)!.status === 'pending' ? 'idle' : nodeById(nodeId)!.status }}
            </span>
          </div>
          <div class="pl-3 font-mono text-[10px] leading-relaxed min-h-[14px]">
            <span v-if="nodeById(nodeId)!.status === 'pending'" class="text-[#2e2852]">_</span>
            <span v-else-if="nodeById(nodeId)!.status === 'failed'" class="text-red-500 line-clamp-2 break-all">{{ nodeById(nodeId)!.error ?? 'error' }}</span>
            <span v-else class="text-[#5c4e94] line-clamp-2 break-words">
              {{ nodeById(nodeId)!.output }}<span v-if="nodeById(nodeId)!.status === 'running'" class="cursor-blink text-cyan-400">█</span>
            </span>
          </div>
        </div>
      </template>

    </div>
  </div>

  <!-- Detail Modal -->
  <Teleport to="body">
    <Transition name="modal-fade">
      <div v-if="selectedNode" class="fixed inset-0 z-[9999] flex items-center justify-center p-4" @click.self="selectedNode = null">
        <div class="absolute inset-0 bg-black/60 backdrop-blur-[3px]" @click="selectedNode = null"/>
        <div class="relative bg-[#0d0c1a] border border-[#2a2347] rounded-[14px] shadow-[0_8px_48px_rgba(0,0,0,0.7)] w-full max-w-[660px] max-h-[78vh] flex flex-col">

          <!-- Modal title bar -->
          <div class="flex items-center justify-between px-5 py-3 bg-[#17152b] border-b border-[#2a2347] rounded-t-[14px] shrink-0">
            <div class="flex items-center gap-2">
              <span class="font-mono text-[10px] text-[#3d365c]">$</span>
              <span class="font-mono text-[13px] font-semibold text-[#b8a8f0]">{{ selectedNode.label }}</span>
              <span
                class="font-mono text-[9px] uppercase tracking-wider px-1.5 py-0.5 rounded"
                :class="{
                  'text-cyan-400 bg-cyan-400/10': selectedNode.status === 'running',
                  'text-emerald-400 bg-emerald-400/10': selectedNode.status === 'completed',
                  'text-red-400 bg-red-400/10': selectedNode.status === 'failed',
                  'text-[#3d365c] bg-[#1a1830]': selectedNode.status === 'pending',
                }"
              >{{ selectedNode.status === 'pending' ? 'idle' : selectedNode.status }}</span>
            </div>
            <div class="flex items-center gap-2">
              <button
                class="font-mono text-[10px] px-2.5 py-1 rounded-[6px] border border-[#2a2347] text-[#6b5aad] hover:text-[#b8a8f0] hover:border-[#6b5aad] transition cursor-pointer"
                @click="copyOutput"
              >{{ copied ? 'copied ✓' : 'copy' }}</button>
              <button class="text-[#3d365c] hover:text-[#b8a8f0] transition cursor-pointer ml-1" @click="selectedNode = null">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                  <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
          </div>

          <!-- Modal output -->
          <div class="flex-1 overflow-y-auto px-5 py-4">
            <pre
              v-if="selectedNode.output"
              class="font-mono text-[12px] text-[#8a7ac4] whitespace-pre-wrap leading-[1.7] m-0 break-words"
            >{{ selectedNode.output }}<span v-if="selectedNode.status === 'running'" class="cursor-blink text-cyan-400">█</span></pre>
            <div v-else-if="selectedNode.status === 'failed'" class="font-mono text-[12px] text-red-400 leading-relaxed">{{ selectedNode.error ?? 'unknown error' }}</div>
            <div v-else class="font-mono text-[11px] text-[#3d365c]">waiting for output...</div>
          </div>

        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.cursor-blink {
  animation: blink 1s step-end infinite;
}
@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}
.modal-fade-enter-active,
.modal-fade-leave-active { transition: opacity 0.15s ease; }
.modal-fade-enter-active .relative,
.modal-fade-leave-active .relative { transition: transform 0.15s ease, opacity 0.15s ease; }
.modal-fade-enter-from,
.modal-fade-leave-to { opacity: 0; }
.modal-fade-enter-from .relative { transform: scale(0.96) translateY(12px); opacity: 0; }
.modal-fade-leave-to .relative { transform: scale(0.96) translateY(12px); opacity: 0; }
</style>
