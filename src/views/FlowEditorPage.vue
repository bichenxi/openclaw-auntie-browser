<script setup lang="ts">
import { VueFlow, useVueFlow, type Node, type Edge } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { useFlowsStore } from '@/stores/flows'
import { useAgentsStore } from '@/stores/agents'
import { listAgents, type AgentInfo } from '@/api/agents'
import type { AgentFlow, FlowNode, FlowEdge } from '@/api/flows'

import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'

const flowsStore = useFlowsStore()
const agentsStore = useAgentsStore()

const agents = ref<AgentInfo[]>([])
const showFlowList = ref(true)
const editingFlow = ref<AgentFlow | null>(null)
const flowName = ref('')
const saving = ref(false)

// VueFlow 节点/边（UI 层）
const vfNodes = ref<Node[]>([])
const vfEdges = ref<Edge[]>([])

const { onConnect, addEdges, onNodesChange, onEdgesChange, applyNodeChanges, applyEdgeChanges } = useVueFlow()

onConnect((conn) => {
  const edge: Edge = {
    id: `e-${conn.source}-${conn.target}`,
    source: conn.source,
    target: conn.target,
    animated: true,
    style: { stroke: '#7c5cfc' },
  }
  addEdges([edge])
})

onNodesChange((changes) => applyNodeChanges(changes))
onEdgesChange((changes) => applyEdgeChanges(changes))

onMounted(async () => {
  await flowsStore.refresh()
  agents.value = await listAgents()
})

// ── 打开 Flow ──────────────────────────────────────────────────────────────

function openFlow(flow: AgentFlow) {
  editingFlow.value = { ...flow }
  flowName.value = flow.name
  showFlowList.value = false
  syncToVueFlow(flow)
}

function createNew() {
  const flow = flowsStore.newFlow()
  editingFlow.value = flow
  flowName.value = flow.name
  showFlowList.value = false
  syncToVueFlow(flow)
}

function backToList() {
  showFlowList.value = true
  editingFlow.value = null
}

// ── VueFlow ↔ AgentFlow 同步 ───────────────────────────────────────────────

function syncToVueFlow(flow: AgentFlow) {
  vfNodes.value = flow.nodes.map(n => ({
    id: n.id,
    type: n.type === 'agent' ? 'default' : n.type === 'start' ? 'input' : 'output',
    label: n.label,
    position: n.position,
    data: { agentWork: n.agent_work },
    style: nodeStyle(n.type),
  }))
  vfEdges.value = flow.edges.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
    animated: true,
    style: { stroke: '#7c5cfc' },
  }))
}

function nodeStyle(type: string) {
  if (type === 'start') return { background: '#e8f5e9', border: '1.5px solid #66bb6a', borderRadius: '10px' }
  if (type === 'end') return { background: '#fce4ec', border: '1.5px solid #ef5350', borderRadius: '10px' }
  return { background: '#f0ecfa', border: '1.5px solid #7c5cfc', borderRadius: '10px', minWidth: '120px' }
}

function collectFlow(): AgentFlow {
  const nodes: FlowNode[] = vfNodes.value.map(n => ({
    id: n.id,
    type: (n.type === 'input' ? 'start' : n.type === 'output' ? 'end' : 'agent') as FlowNode['type'],
    label: String(n.label ?? ''),
    agent_work: n.data?.agentWork,
    position: n.position,
  }))
  const edges: FlowEdge[] = vfEdges.value.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
  }))
  return { ...editingFlow.value!, name: flowName.value, nodes, edges }
}

// ── 添加 Agent 节点 ────────────────────────────────────────────────────────

function addAgentNode(agent: AgentInfo) {
  const id = `agent-${agent.name}-${Date.now()}`
  vfNodes.value = [...vfNodes.value, {
    id,
    type: 'default',
    label: agent.name,
    position: { x: 200 + Math.random() * 200, y: 150 + Math.random() * 150 },
    data: { agentWork: agent.name },
    style: nodeStyle('agent'),
  }]
}

// ── 保存 ───────────────────────────────────────────────────────────────────

async function save() {
  saving.value = true
  try {
    const flow = collectFlow()
    const saved = await flowsStore.save(flow)
    editingFlow.value = saved
  } finally {
    saving.value = false
  }
}

async function removeFlow(flow_id: string) {
  await flowsStore.remove(flow_id)
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#faf9ff]">

    <!-- 页头 -->
    <div class="flex items-center justify-between px-8 py-5 border-b border-[#e8e2f4] bg-white shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-9 h-9 rounded-[10px] bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] flex-center shadow">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="5" cy="12" r="2" /><circle cx="19" cy="5" r="2" /><circle cx="19" cy="19" r="2" />
            <line x1="7" y1="12" x2="17" y2="6" /><line x1="7" y1="12" x2="17" y2="18" />
          </svg>
        </div>
        <div>
          <div class="text-[17px] font-bold text-[#1f1f2e]">
            <template v-if="showFlowList">Agent Flow 编排</template>
            <template v-else>
              <input
                v-model="flowName"
                class="text-[17px] font-bold text-[#1f1f2e] bg-transparent border-none outline-none w-[220px]"
                placeholder="工作流名称"
              />
            </template>
          </div>
          <div class="text-[12px] text-[#9b8ec4]">
            {{ showFlowList ? '管理多 Agent 协作工作流' : '拖拽连接 Agent 节点，构建协作流程' }}
          </div>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <template v-if="showFlowList">
          <button
            class="flex items-center gap-2 px-4 py-2 text-[13px] font-medium text-white rounded-[10px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] hover:shadow-[0_4px_14px_rgba(95,71,206,0.3)] active:scale-[0.97]"
            @click="createNew"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            新建工作流
          </button>
        </template>
        <template v-else>
          <button
            class="px-4 py-2 text-[13px] rounded-[8px] border border-[#e8e2f4] text-[#6b5f8a] bg-white hover:bg-[#f5f3ff] cursor-pointer transition"
            @click="backToList"
          >
            返回列表
          </button>
          <button
            class="flex items-center gap-2 px-4 py-2 text-[13px] font-medium text-white rounded-[10px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)] disabled:opacity-50"
            :disabled="saving"
            @click="save"
          >
            <svg v-if="saving" class="animate-spin" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 12a9 9 0 1 1-6.219-8.56" /></svg>
            {{ saving ? '保存中...' : '保存' }}
          </button>
        </template>
      </div>
    </div>

    <!-- Flow 列表 -->
    <div v-if="showFlowList" class="flex-1 overflow-y-auto px-8 py-6">
      <div v-if="flowsStore.loading" class="flex-center h-40">
        <span class="w-7 h-7 border-[2.5px] border-secondary border-t-transparent rounded-full animate-spin" />
      </div>

      <div v-else-if="flowsStore.flows.length === 0" class="flex flex-col items-center gap-4 py-16">
        <div class="w-16 h-16 rounded-2xl bg-[#f0ecfa] flex-center">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#9b8ec4" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="5" cy="12" r="2" /><circle cx="19" cy="5" r="2" /><circle cx="19" cy="19" r="2" />
            <line x1="7" y1="12" x2="17" y2="6" /><line x1="7" y1="12" x2="17" y2="18" />
          </svg>
        </div>
        <p class="text-[14px] text-[#9b8ec4] m-0">还没有工作流</p>
        <button
          class="mt-2 px-5 py-2 text-[13px] font-medium rounded-[10px] border border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12 cursor-pointer transition"
          @click="createNew"
        >
          创建第一个工作流
        </button>
      </div>

      <div v-else class="grid grid-cols-1 gap-3 max-w-[680px]">
        <div
          v-for="flow in flowsStore.flows"
          :key="flow.id"
          class="flex items-center gap-4 px-5 py-4 bg-white rounded-xl border border-[#e8e2f4] hover:border-secondary/30 hover:shadow-[0_2px_12px_rgba(95,71,206,0.08)] transition cursor-pointer group"
          @click="openFlow(flow)"
        >
          <div class="w-10 h-10 rounded-[10px] bg-[linear-gradient(135deg,#f0ecfa_0%,#e4dcf7_100%)] flex-center shrink-0">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#7c5cfc" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="5" cy="12" r="2" /><circle cx="19" cy="5" r="2" /><circle cx="19" cy="19" r="2" />
              <line x1="7" y1="12" x2="17" y2="6" /><line x1="7" y1="12" x2="17" y2="18" />
            </svg>
          </div>
          <div class="flex-1 min-w-0">
            <div class="text-[14px] font-semibold text-[#1f1f2e]">{{ flow.name }}</div>
            <div class="text-[11px] text-[#9b8ec4] mt-0.5">{{ flow.nodes.length }} 个节点 · {{ flow.edges.length }} 条连线</div>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <button
              class="px-2.5 py-1 text-[11px] rounded-full border border-red-200 text-red-400 bg-red-50 hover:bg-red-100 cursor-pointer transition opacity-0 group-hover:opacity-100"
              @click.stop="removeFlow(flow.id)"
            >删除</button>
            <svg class="text-[#c4bdd8] group-hover:text-secondary transition" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="9 18 15 12 9 6" />
            </svg>
          </div>
        </div>
      </div>
    </div>

    <!-- Flow 编辑器 -->
    <div v-else class="flex flex-1 min-h-0">
      <!-- 左侧 Agent 面板 -->
      <div class="w-52 shrink-0 border-r border-[#e8e2f4] bg-white flex flex-col overflow-hidden">
        <div class="px-4 py-3 text-[11px] font-semibold text-[#9b8ec4] uppercase tracking-wider border-b border-[#f0ecfa]">
          可用 Agent
        </div>
        <div class="flex-1 overflow-y-auto py-2">
          <div
            v-for="agent in agents"
            :key="agent.name"
            class="flex items-center gap-2.5 px-4 py-2.5 cursor-pointer hover:bg-[#f5f3ff] transition group"
            @click="addAgentNode(agent)"
          >
            <div class="w-7 h-7 rounded-[7px] bg-[linear-gradient(135deg,#f0ecfa_0%,#e4dcf7_100%)] flex-center shrink-0">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#7c5cfc" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
              </svg>
            </div>
            <span class="text-[12px] text-[#3d3558] truncate">{{ agent.name }}</span>
            <svg class="ml-auto text-[#c4bdd8] opacity-0 group-hover:opacity-100 transition shrink-0" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" /></svg>
          </div>
          <div v-if="agents.length === 0" class="px-4 py-3 text-[11px] text-[#b8b0cc]">
            暂无 Agent，请先在「智能体」页面创建
          </div>
        </div>
      </div>

      <!-- VueFlow 画布 -->
      <div class="flex-1 min-w-0">
        <VueFlow
          v-model:nodes="vfNodes"
          v-model:edges="vfEdges"
          fit-view-on-init
          :default-edge-options="{ animated: true, style: { stroke: '#7c5cfc' } }"
        >
          <Background pattern-color="#e8e2f4" :gap="20" />
          <Controls />
        </VueFlow>
      </div>
    </div>
  </div>
</template>
