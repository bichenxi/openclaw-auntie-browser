<script setup lang="ts">
import { VueFlow, useVueFlow, type Node, type Edge } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import { Controls } from '@vue-flow/controls'
import { useFlowsStore } from '@/stores/flows'
import { useSettingsStore } from '@/stores/settings'
import { useTabsStore } from '@/stores/tabs'
import { useOpenclawStore } from '@/stores/openclaw'
import { listAgents, type AgentInfo } from '@/api/agents'
import { runFlowNode } from '@/api/flows'
import type { AgentFlow, FlowNode, FlowEdge } from '@/api/flows'

import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import '@vue-flow/controls/dist/style.css'

const flowsStore = useFlowsStore()
const settingsStore = useSettingsStore()
const tabsStore = useTabsStore()
const ocStore = useOpenclawStore()

const agents = ref<AgentInfo[]>([])
const showFlowList = ref(true)
const editingFlow = ref<AgentFlow | null>(null)
const flowName = ref('')

// 保存状态
const saveStatus = ref<'idle' | 'ok' | 'err'>('idle')
let saveTimer: ReturnType<typeof setTimeout> | null = null

// 执行状态
const runDialogVisible = ref(false)
const initialTask = ref('')
const running = ref(false)
const nodeStatuses = ref<Record<string, 'idle' | 'running' | 'completed' | 'failed'>>({})

// VueFlow
const vfNodes = ref<Node[]>([])
const vfEdges = ref<Edge[]>([])

const { onConnect, addEdges, onNodesChange, onEdgesChange, applyNodeChanges, applyEdgeChanges } = useVueFlow()

onConnect((conn) => {
  addEdges([{
    id: `e-${conn.source}-${conn.target}`,
    source: conn.source,
    target: conn.target,
    animated: true,
    style: { stroke: '#7c5cfc' },
  } as Edge])
})
onNodesChange((changes) => applyNodeChanges(changes))
onEdgesChange((changes) => applyEdgeChanges(changes))

onMounted(async () => {
  await flowsStore.refresh()
  agents.value = await listAgents()
})

// ── 列表操作 ───────────────────────────────────────────────────────────────

function openFlow(flow: AgentFlow) {
  editingFlow.value = { ...flow }
  flowName.value = flow.name
  nodeStatuses.value = {}
  showFlowList.value = false
  syncToVueFlow(flow)
}

function createNew() {
  const flow = flowsStore.newFlow()
  editingFlow.value = flow
  flowName.value = flow.name
  nodeStatuses.value = {}
  showFlowList.value = false
  syncToVueFlow(flow)
}

function backToList() {
  showFlowList.value = true
  editingFlow.value = null
}

// ── VueFlow ↔ AgentFlow ────────────────────────────────────────────────────

function syncToVueFlow(flow: AgentFlow) {
  vfNodes.value = flow.nodes.map(n => ({
    id: n.id,
    type: n.type === 'agent' ? 'default' : n.type === 'start' ? 'input' : 'output',
    label: n.label,
    position: n.position,
    data: { agentWork: n.agent_work },
    style: nodeStyle(n.type, 'idle'),
  }))
  vfEdges.value = flow.edges.map(e => ({
    id: e.id,
    source: e.source,
    target: e.target,
    animated: true,
    style: { stroke: '#7c5cfc' },
  }))
}

function nodeStyle(type: string, status: string = 'idle') {
  const bg: Record<string, string> = {
    running: '#dbeafe', completed: '#dcfce7', failed: '#fee2e2',
    idle: type === 'start' ? '#e8f5e9' : type === 'end' ? '#fce4ec' : '#f0ecfa',
  }
  const border: Record<string, string> = {
    running: '#3b82f6', completed: '#22c55e', failed: '#ef4444',
    idle: type === 'start' ? '#66bb6a' : type === 'end' ? '#ef5350' : '#7c5cfc',
  }
  return {
    background: bg[status] ?? bg.idle,
    border: `1.5px solid ${border[status] ?? border.idle}`,
    borderRadius: '10px', minWidth: '120px',
  }
}

function setNodeVisualStatus(nodeId: string, type: string, status: 'idle' | 'running' | 'completed' | 'failed') {
  const node = vfNodes.value.find(n => n.id === nodeId)
  if (node) node.style = nodeStyle(type, status)
  nodeStatuses.value[nodeId] = status
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
    id: e.id, source: e.source, target: e.target,
  }))
  return { ...editingFlow.value!, name: flowName.value, nodes, edges }
}

function addAgentNode(agent: AgentInfo) {
  const id = `agent-${agent.name}-${Date.now()}`
  vfNodes.value = [...vfNodes.value, {
    id, type: 'default', label: agent.name,
    position: { x: 200 + Math.random() * 200, y: 150 + Math.random() * 150 },
    data: { agentWork: agent.name },
    style: nodeStyle('agent'),
  }]
}

// ── 保存 ───────────────────────────────────────────────────────────────────

async function save() {
  if (saveTimer) clearTimeout(saveTimer)
  saveStatus.value = 'idle'
  try {
    const saved = await flowsStore.save(collectFlow())
    editingFlow.value = saved
    saveStatus.value = 'ok'
  } catch {
    saveStatus.value = 'err'
  }
  saveTimer = setTimeout(() => { saveStatus.value = 'idle' }, 2000)
}

async function removeFlow(flowId: string) {
  await flowsStore.remove(flowId)
}

// ── 执行（结果在对话中展示） ────────────────────────────────────────────────

/**
 * Kahn BFS 按层次分组节点：同层节点可并行执行
 * 返回 [[level0 agents], [level1 agents], ...]
 */
function getExecutionLevels(flow: AgentFlow): FlowNode[][] {
  const nodeMap = new Map(flow.nodes.map(n => [n.id, n]))
  const inDegree = new Map<string, number>()
  const childrenOf = new Map<string, string[]>()
  for (const n of flow.nodes) { inDegree.set(n.id, 0); childrenOf.set(n.id, []) }
  for (const e of flow.edges) {
    inDegree.set(e.target, (inDegree.get(e.target) ?? 0) + 1)
    childrenOf.get(e.source)?.push(e.target)
  }

  const levels: FlowNode[][] = []
  const visited = new Set<string>()
  let frontier = flow.nodes.filter(n => (inDegree.get(n.id) ?? 0) === 0)

  while (frontier.length > 0) {
    const agentLevel = frontier.filter(n => n.type === 'agent')
    if (agentLevel.length > 0) levels.push(agentLevel)
    const next: FlowNode[] = []
    for (const n of frontier) {
      visited.add(n.id)
      for (const cid of childrenOf.get(n.id) ?? []) {
        const deg = (inDegree.get(cid) ?? 0) - 1
        inDegree.set(cid, deg)
        if (deg === 0 && !visited.has(cid)) {
          const child = nodeMap.get(cid)
          if (child) next.push(child)
        }
      }
    }
    frontier = next
  }
  return levels
}

const executionLevels = computed(() =>
  editingFlow.value ? getExecutionLevels(editingFlow.value) : []
)

/** 为节点构建完整 prompt：原始任务 + 自己的角色 + 前驱输出 */
function buildNodePrompt(
  node: FlowNode,
  flow: AgentFlow,
  outputs: Map<string, string>,
  initialTask: string,
): string {
  const sourceOutputs = flow.edges
    .filter(e => e.target === node.id)
    .map(e => {
      const src = flow.nodes.find(n => n.id === e.source)
      const out = outputs.get(e.source)
      return out ? { label: src?.label ?? e.source, text: out } : null
    })
    .filter(Boolean) as { label: string; text: string }[]

  const parts: string[] = []
  parts.push(`【总体任务】\n${initialTask}`)
  parts.push(`【你的角色】\n你是"${node.label}"，在工作流中负责你的专属环节，请围绕总体任务完成你的部分。`)

  if (sourceOutputs.length === 1) {
    parts.push(`【上游输出（来自「${sourceOutputs[0].label}」）】\n${sourceOutputs[0].text}`)
  } else if (sourceOutputs.length > 1) {
    const combined = sourceOutputs.map(s => `— 来自「${s.label}」：\n${s.text}`).join('\n\n')
    parts.push(`【上游输出】\n${combined}`)
  }

  return parts.join('\n\n')
}

async function startRun() {
  if (!editingFlow.value || !initialTask.value.trim()) return
  const flow = editingFlow.value
  const levels = getExecutionLevels(flow)
  const token = settingsStore.bearerToken

  if (!token) {
    ocStore.messages.push({ type: 'user', text: '❌ 未配置 Bearer Token，请在设置页面填写', streaming: false })
    tabsStore.switchToSpecialView('openclaw')
    return
  }
  if (levels.length === 0) {
    ocStore.messages.push({ type: 'user', text: '⚠️ 工作流没有 Agent 节点', streaming: false })
    tabsStore.switchToSpecialView('openclaw')
    return
  }

  runDialogVisible.value = false
  running.value = true

  // 切换到对话视图
  tabsStore.switchToSpecialView('openclaw')

  // 屏蔽 stream-item 写入主聊天（执行卡片自行展示进度）
  ocStore.suppressStream = true

  // 创建执行状态，推入一条 flow 类型消息（渲染为卡片）
  const execId = ocStore.createFlowExecution(
    flow.name,
    initialTask.value,
    levels.map(lvl => lvl.map(n => ({ id: n.id, label: n.label }))),
  )
  ocStore.messages.push({ type: 'flow', text: '', streaming: false, executionId: execId })

  const nodeOutputs = new Map<string, string>()
  const baseUrlOpt = settingsStore.baseUrl ? { baseUrl: settingsStore.baseUrl } : {}
  let finalOutput = ''
  let allSucceeded = true

  for (const levelNodes of levels) {
    if (levelNodes.length === 1) {
      // ── 串行单节点 ──
      const node = levelNodes[0]
      const input = buildNodePrompt(node, flow, nodeOutputs, initialTask.value)
      setNodeVisualStatus(node.id, 'agent', 'running')
      ocStore.updateFlowNode(execId, node.id, { status: 'running' })

      try {
        const output = await runFlowNode({
          token,
          sessionKey: `agent:${node.agent_work}:${node.agent_work}`,
          input,
          ...baseUrlOpt,
        })
        setNodeVisualStatus(node.id, 'agent', 'completed')
        nodeOutputs.set(node.id, output)
        finalOutput = output
        ocStore.updateFlowNode(execId, node.id, { status: 'completed', output })
      } catch (err: unknown) {
        const errMsg = err instanceof Error ? err.message : String(err)
        setNodeVisualStatus(node.id, 'agent', 'failed')
        ocStore.updateFlowNode(execId, node.id, { status: 'failed', error: errMsg })
        allSucceeded = false
        break
      }
    } else {
      // ── 并行多节点 ──
      levelNodes.forEach(n => {
        setNodeVisualStatus(n.id, 'agent', 'running')
        ocStore.updateFlowNode(execId, n.id, { status: 'running' })
      })

      const results = await Promise.allSettled(
        levelNodes.map(node =>
          runFlowNode({
            token,
            sessionKey: `agent:${node.agent_work}:${node.agent_work}`,
            input: buildNodePrompt(node, flow, nodeOutputs, initialTask.value),
            ...baseUrlOpt,
          })
        )
      )

      let anyFailed = false
      const parallelOutputs: string[] = []
      for (let i = 0; i < results.length; i++) {
        const res = results[i]
        const node = levelNodes[i]
        if (res.status === 'fulfilled') {
          setNodeVisualStatus(node.id, 'agent', 'completed')
          nodeOutputs.set(node.id, res.value)
          parallelOutputs.push(res.value)
          ocStore.updateFlowNode(execId, node.id, { status: 'completed', output: res.value })
        } else {
          const errMsg = res.reason instanceof Error ? res.reason.message : String(res.reason)
          setNodeVisualStatus(node.id, 'agent', 'failed')
          ocStore.updateFlowNode(execId, node.id, { status: 'failed', error: errMsg })
          anyFailed = true
          allSucceeded = false
        }
      }
      if (parallelOutputs.length > 0) finalOutput = parallelOutputs[parallelOutputs.length - 1]
      if (anyFailed) break
    }
  }

  ocStore.finishFlowExecution(execId, allSucceeded ? 'completed' : 'failed')
  ocStore.suppressStream = false
  running.value = false

  // 最终结果以普通对话形式展示
  if (allSucceeded && finalOutput) {
    ocStore.messages.push({ type: 'thought', text: finalOutput, streaming: false })
  }
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
              <input v-model="flowName" class="text-[17px] font-bold text-[#1f1f2e] bg-transparent border-none outline-none w-[220px]" placeholder="工作流名称" />
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
          <button class="px-4 py-2 text-[13px] rounded-[8px] border border-[#e8e2f4] text-[#6b5f8a] bg-white hover:bg-[#f5f3ff] cursor-pointer transition" @click="backToList">
            返回列表
          </button>
          <button
            class="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[8px] border border-emerald-300 text-emerald-600 bg-emerald-50 hover:bg-emerald-100 cursor-pointer transition disabled:opacity-50"
            :disabled="running"
            @click="runDialogVisible = true"
          >
            <svg v-if="running" class="animate-spin" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M21 12a9 9 0 1 1-6.219-8.56" /></svg>
            <svg v-else width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polygon points="5 3 19 12 5 21 5 3" />
            </svg>
            {{ running ? '执行中...' : '运行' }}
          </button>
          <button
            class="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium text-white rounded-[10px] cursor-pointer transition bg-[linear-gradient(135deg,#7c5cfc_0%,#5f47ce_100%)] shadow-[0_2px_8px_rgba(95,71,206,0.2)]"
            @click="save"
          >
            <svg v-if="saveStatus === 'ok'" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
            <svg v-else-if="saveStatus === 'err'" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg>
            {{ saveStatus === 'ok' ? '已保存' : saveStatus === 'err' ? '保存失败' : '保存' }}
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
        <button class="mt-2 px-5 py-2 text-[13px] font-medium rounded-[10px] border border-secondary/30 text-secondary bg-secondary/6 hover:bg-secondary/12 cursor-pointer transition" @click="createNew">
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
        <div class="px-4 py-3 text-[11px] font-semibold text-[#9b8ec4] uppercase tracking-wider border-b border-[#f0ecfa]">可用 Agent</div>
        <div class="flex-1 overflow-y-auto py-2">
          <div
            v-for="agent in agents"
            :key="agent.name"
            class="flex items-center gap-2.5 px-4 py-2.5 cursor-pointer hover:bg-[#f5f3ff] transition group"
            @click="addAgentNode(agent)"
          >
            <div class="w-7 h-7 rounded-[7px] bg-[linear-gradient(135deg,#f0ecfa_0%,#e4dcf7_100%)] flex-center shrink-0 relative">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#7c5cfc" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" /><circle cx="9" cy="7" r="4" />
              </svg>
              <!-- 执行状态小点 -->
              <span
                v-if="editingFlow?.nodes.find(n => n.agent_work === agent.name)"
                class="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full border border-white"
                :class="{
                  'bg-blue-400': Object.entries(nodeStatuses).find(([id]) => editingFlow?.nodes.find(n => n.id === id && n.agent_work === agent.name))?.[1] === 'running',
                  'bg-emerald-400': Object.entries(nodeStatuses).find(([id]) => editingFlow?.nodes.find(n => n.id === id && n.agent_work === agent.name))?.[1] === 'completed',
                  'bg-red-400': Object.entries(nodeStatuses).find(([id]) => editingFlow?.nodes.find(n => n.id === id && n.agent_work === agent.name))?.[1] === 'failed',
                  'bg-[#c4bdd8]': !Object.entries(nodeStatuses).find(([id]) => editingFlow?.nodes.find(n => n.id === id && n.agent_work === agent.name)),
                }"
              />
            </div>
            <span class="text-[12px] text-[#3d3558] truncate">{{ agent.name }}</span>
            <svg class="ml-auto text-[#c4bdd8] opacity-0 group-hover:opacity-100 transition shrink-0" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
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

  <!-- 运行对话框 -->
  <Teleport to="body">
    <Transition name="overlay">
      <div v-if="runDialogVisible" class="fixed inset-0 z-[9999] flex items-center justify-center" @click.self="runDialogVisible = false">
        <div class="absolute inset-0 bg-black/30 backdrop-blur-sm" />
        <div class="relative bg-white rounded-2xl shadow-2xl w-full max-w-[480px] mx-4 overflow-hidden">
          <div class="flex items-center gap-3 px-6 pt-6 pb-4 border-b border-[#f0ecfa]">
            <div class="w-8 h-8 rounded-[9px] bg-emerald-100 flex-center shrink-0">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 3 19 12 5 21 5 3" /></svg>
            </div>
            <div>
              <div class="text-[15px] font-bold text-[#1f1f2e]">运行工作流</div>
              <div class="text-[11px] text-[#9b8ec4] mt-0.5">结果将在 OpenClaw 对话中展示</div>
            </div>
          </div>

          <div class="px-6 py-5 flex flex-col gap-4">
            <!-- 执行计划预览 -->
            <div>
              <div class="text-[11px] font-medium text-[#9b8ec4] mb-2">执行计划</div>
              <div class="flex flex-wrap gap-1.5 items-center">
                <template v-for="(lvl, li) in executionLevels" :key="li">
                  <!-- 并行组 -->
                  <div v-if="lvl.length > 1" class="flex gap-1 items-center px-2 py-1 rounded-lg bg-amber-50 border border-amber-200">
                    <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#d97706" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>
                    <span v-for="node in lvl" :key="node.id" class="text-[11px] text-amber-700">{{ node.label }}</span>
                  </div>
                  <!-- 单节点 -->
                  <span v-else class="px-2.5 py-1 text-[11px] rounded-full bg-[#f0ecfa] text-[#5f47ce] border border-[#e4dcf7]">{{ lvl[0].label }}</span>
                  <!-- 箭头 -->
                  <svg v-if="li < executionLevels.length - 1" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#c4bdd8" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12" /><polyline points="12 5 19 12 12 19" /></svg>
                </template>
                <span v-if="executionLevels.length === 0" class="text-[11px] text-[#b8b0cc]">没有 Agent 节点，请先添加</span>
              </div>
            </div>

            <!-- 初始任务输入 -->
            <div>
              <label class="text-[12px] font-medium text-[#6b5f8a] block mb-1.5">初始任务描述</label>
              <textarea
                v-model="initialTask"
                rows="4"
                class="block w-full box-border px-4 py-3 text-[13px] border border-[#e8e2f4] rounded-xl outline-none focus:border-secondary focus:shadow-[0_0_0_3px_rgba(95,71,206,0.08)] resize-none"
                placeholder="描述你希望 Agent 完成的任务..."
                @keydown.meta.enter="startRun"
              />
            </div>
          </div>

          <div class="flex gap-3 justify-end px-6 py-4 bg-[#faf9ff] border-t border-[#f0ecfa]">
            <button class="px-4 py-2 text-[13px] rounded-[8px] border border-[#e8e2f4] text-[#6b5f8a] bg-white hover:bg-[#f5f3ff] cursor-pointer transition" @click="runDialogVisible = false">取消</button>
            <button
              class="px-5 py-2 text-[13px] font-medium text-white rounded-[8px] cursor-pointer transition bg-emerald-500 hover:bg-emerald-600 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!initialTask.trim() || executionLevels.length === 0"
              @click="startRun"
            >开始执行</button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.overlay-enter-active, .overlay-leave-active { transition: opacity 0.15s ease; }
.overlay-enter-active > div:last-child, .overlay-leave-active > div:last-child { transition: transform 0.15s ease, opacity 0.15s ease; }
.overlay-enter-from, .overlay-leave-to { opacity: 0; }
.overlay-enter-from > div:last-child { transform: scale(0.97) translateY(8px); opacity: 0; }
.overlay-leave-to > div:last-child { transform: scale(0.97) translateY(8px); opacity: 0; }
</style>
