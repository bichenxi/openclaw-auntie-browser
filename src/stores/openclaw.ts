import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'

export type MessageType = 'thought' | 'tool' | 'user' | 'flow'

export interface Message {
  type: MessageType
  text: string
  streaming: boolean
  executionId?: string
}

export interface FlowNodeState {
  id: string
  label: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  output: string
  error?: string
}

export interface FlowExecutionState {
  id: string
  flowName: string
  task: string
  status: 'running' | 'completed' | 'failed'
  nodes: FlowNodeState[]
  /** 并行分支，每个分支是从 start 出发的独立节点链（列显示） */
  branches: string[][]
  /** 分支汇聚后的顺序节点（全宽显示） */
  convergeIds: string[]
}

export const useOpenclawStore = defineStore('openclaw', () => {
  const messages = ref<Message[]>([])
  const sending = ref(false)
  const sendError = ref('')

  /** 所有工作流执行状态，reactive 以驱动卡片更新 */
  const flowExecutions = ref<Record<string, FlowExecutionState>>({})

  let listenersStarted = false

  function startListeners() {
    if (listenersStarted) return
    listenersStarted = true

    listen<{ nodeId: string; text: string }>('flow-stream-item', (e) => {
      const { nodeId, text } = e.payload
      for (const exec of Object.values(flowExecutions.value)) {
        const node = exec.nodes.find(n => n.id === nodeId)
        if (node) { node.output += text; break }
      }
    })

    listen<{ type: string; text: string }>('stream-item', (e) => {
      const payload = e.payload
      if (!payload?.type || !payload?.text) return
      const type: MessageType = payload.type === 'tool' ? 'tool' : 'thought'
      const last = messages.value[messages.value.length - 1]
      if (last && last.streaming && last.type === type) {
        last.text += payload.text
      } else {
        messages.value.push({ type, text: payload.text, streaming: true })
      }
    })

    listen('stream-done', () => {
      const last = messages.value[messages.value.length - 1]
      if (last && last.streaming) last.streaming = false
      sending.value = false
    })
  }

  function createFlowExecution(
    flowName: string,
    task: string,
    allNodes: { id: string; label: string }[],
    branches: string[][],
    convergeIds: string[],
  ): string {
    const id = `fexec-${Date.now()}`
    flowExecutions.value[id] = {
      id,
      flowName,
      task,
      status: 'running',
      nodes: allNodes.map(n => ({ id: n.id, label: n.label, status: 'pending', output: '' })),
      branches,
      convergeIds,
    }
    return id
  }

  function updateFlowNode(execId: string, nodeId: string, patch: Partial<FlowNodeState>) {
    const exec = flowExecutions.value[execId]
    if (!exec) return
    const node = exec.nodes.find(n => n.id === nodeId)
    if (node) Object.assign(node, patch)
  }

  function finishFlowExecution(execId: string, status: 'completed' | 'failed') {
    const exec = flowExecutions.value[execId]
    if (exec) exec.status = status
  }

  return {
    messages, sending, sendError,
    flowExecutions,
    startListeners,
    createFlowExecution, updateFlowNode, finishFlowExecution,
  }
})
