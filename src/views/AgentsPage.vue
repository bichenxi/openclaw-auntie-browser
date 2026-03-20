<script setup lang="ts">
import { useAgentsStore } from '@/stores/agents'
import { useTabsStore } from '@/stores/tabs'
import { listAgents, type AgentInfo } from '@/api/agents'
import AgentAddWizard from '@/components/AgentAddWizard.vue'

const agentsStore = useAgentsStore()
const tabsStore = useTabsStore()
const agents = ref<AgentInfo[]>([])
const loading = ref(false)
const showAddDialog = ref(false)
const newWorkName = ref('')
const addError = ref('')

async function refresh() {
  loading.value = true
  try {
    agents.value = await listAgents()
  } catch {
    // 忽略
  } finally {
    loading.value = false
  }
}

onMounted(refresh)

function openAddDialog() {
  newWorkName.value = ''
  addError.value = ''
  showAddDialog.value = true
}

function confirmAdd() {
  const name = newWorkName.value.trim()
  if (!name) {
    addError.value = '请输入智能体名称'
    return
  }
  if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
    addError.value = '名称只能包含字母、数字、- 和 _'
    return
  }
  showAddDialog.value = false
  agentsStore.openWizard(name)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') confirmAdd()
}

// 向导关闭后刷新列表
watch(() => agentsStore.wizardVisible, (visible) => {
  if (!visible) refresh()
})

function openEditor(agent: AgentInfo) {
  agentsStore.editingWork = agent.name
  tabsStore.switchToSpecialView('agent-editor')
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#faf9ff]">
    <!-- 页头 -->
    <div class="flex items-center justify-between px-8 py-5 border-b border-[#e8e2f4] bg-white shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-[10px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        </div>
        <div>
          <div class="text-[17px] font-bold text-[#1f1f2e]">智能体助手</div>
          <div class="text-[12px] text-[#9b8ec4]">管理 OpenClaw 智能体配置</div>
        </div>
      </div>
      <button
        class="flex items-center gap-2 px-4 py-2 text-[13px] font-medium text-white rounded-[10px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] hover:shadow-[0_4px_14px_rgba(95,71,206,0.3)] active:scale-[0.97]"
        @click="openAddDialog"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
        </svg>
        添加智能体
      </button>
    </div>

    <!-- 内容区 -->
    <div class="flex-1 overflow-y-auto px-8 py-6">
      <!-- 加载 -->
      <div v-if="loading" class="flex-center h-40">
        <span class="w-7 h-7 border-[2.5px] border-secondary border-t-transparent rounded-full animate-spin" />
      </div>

      <!-- 空状态 -->
      <div v-else-if="agents.length === 0" class="flex flex-col items-center gap-4 py-16">
        <div class="w-16 h-16 rounded-2xl bg-[#f0ecfa] flex-center">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#9b8ec4" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        </div>
        <p class="text-[14px] text-[#9b8ec4] text-center m-0">还没有智能体</p>
        <p class="text-[12px] text-[#b8b0cc] text-center m-0">点击右上角「添加智能体」开始配置</p>
        <button
          class="mt-2 px-5 py-2 text-[13px] font-medium rounded-[10px] border border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12 cursor-pointer transition"
          @click="openAddDialog"
        >
          添加第一个智能体
        </button>
      </div>

      <!-- 智能体列表 -->
      <div v-else class="grid grid-cols-1 gap-3 max-w-[680px]">
        <div
          v-for="agent in agents"
          :key="agent.name"
          class="flex items-center gap-4 px-5 py-4 bg-white rounded-xl border border-[#e8e2f4] hover:border-secondary/30 hover:shadow-[0_2px_12px_rgba(95,71,206,0.08)] transition cursor-pointer group"
          @click="openEditor(agent)"
        >
          <div class="w-10 h-10 rounded-[10px] bg-[linear-gradient(135deg,#f0ecfa_0%,#e4dcf7_100%)] flex-center shrink-0">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#7c5cfc" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
              <circle cx="9" cy="7" r="4" />
              <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
              <path d="M16 3.13a4 4 0 0 1 0 7.75" />
            </svg>
          </div>
          <div class="flex-1 min-w-0">
            <div class="text-[14px] font-semibold text-[#1f1f2e]">{{ agent.name }}</div>
            <div class="text-[11px] text-[#9b8ec4] truncate mt-0.5">~/.openclaw/{{ agent.workspace ? `workspace-${agent.workspace}` : 'workspace' }}</div>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <span class="px-2.5 py-1 text-[11px] font-medium rounded-full bg-emerald-50 text-emerald-600 border border-emerald-200">已配置</span>
            <svg class="text-[#c4bdd8] group-hover:text-secondary transition" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </div>
        </div>
      </div>
    </div>

    <!-- 命令提示条 -->
    <div class="px-8 py-3 border-t border-[#e8e2f4] bg-white shrink-0 flex items-center gap-2">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#9b8ec4" stroke-width="2"><polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" /></svg>
      <span class="text-[11px] text-[#9b8ec4] font-mono">openclaw agents add &lt;work&gt;</span>
      <span class="text-[11px] text-[#b8b0cc]">— 在终端中也可直接运行</span>
    </div>
  </div>

  <!-- 添加对话框 -->
  <Teleport to="body">
    <Transition name="overlay">
      <div
        v-if="showAddDialog"
        class="fixed inset-0 z-[9999] flex items-center justify-center"
        @click.self="showAddDialog = false"
      >
        <div class="absolute inset-0 bg-black/30 backdrop-blur-sm" />
        <div class="relative bg-white rounded-2xl shadow-2xl w-full max-w-[400px] mx-4 overflow-hidden">
          <!-- 对话框头部 -->
          <div class="flex items-center gap-3 px-6 pt-6 pb-4 border-b border-[#f0ecfa]">
            <div class="w-8 h-8 rounded-[9px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow shrink-0">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2.5"><line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" /></svg>
            </div>
            <div class="text-[15px] font-bold text-[#1f1f2e]">添加智能体</div>
          </div>

          <!-- 表单 -->
          <div class="px-6 py-5 flex flex-col gap-3">
            <label class="text-[12px] font-medium text-[#6b5f8a]">智能体名称 (work)</label>
            <input
              v-model="newWorkName"
              type="text"
              class="block w-full box-border px-4 py-3 text-[13px] border border-[#e8e2f4] rounded-xl outline-none focus:border-secondary focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)]"
              placeholder="例如：main、assistant、researcher"
              autocomplete="off"
              @keydown="handleKeydown"
            />
            <p v-if="addError" class="text-[11px] text-red-500 m-0">{{ addError }}</p>
            <p class="text-[11px] text-[#b8b0cc] m-0 truncate">
              将运行 <code class="bg-[#f0ecfa] px-1 rounded font-mono">openclaw agents add {{ newWorkName || '&lt;work&gt;' }}</code>
            </p>
          </div>

          <!-- 底部按钮 -->
          <div class="flex gap-3 justify-end px-6 py-4 bg-[#faf9ff] border-t border-[#f0ecfa]">
            <button
              type="button"
              class="px-4 py-2 text-[13px] rounded-[8px] border border-[#e8e2f4] text-[#6b5f8a] bg-white hover:bg-[#f5f3ff] cursor-pointer transition"
              @click="showAddDialog = false"
            >
              取消
            </button>
            <button
              type="button"
              class="px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!newWorkName.trim()"
              @click="confirmAdd"
            >
              确认
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>

  <!-- 向导 -->
  <AgentAddWizard />
</template>

<style scoped>
.overlay-enter-active,
.overlay-leave-active { transition: opacity 0.15s ease; }
.overlay-enter-active > div:last-child,
.overlay-leave-active > div:last-child { transition: transform 0.15s ease, opacity 0.15s ease; }
.overlay-enter-from,
.overlay-leave-to { opacity: 0; }
.overlay-enter-from > div:last-child { transform: scale(0.97) translateY(8px); opacity: 0; }
.overlay-leave-to > div:last-child { transform: scale(0.97) translateY(8px); opacity: 0; }
</style>
