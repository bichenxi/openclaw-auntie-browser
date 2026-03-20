import { defineStore } from 'pinia'
import {
  listFlows, loadFlow, saveFlow, deleteFlow,
  initFlowExecution, updateNodeStatus, setNodeOutput,
  appendFlowLog, finishFlowExecution, getFlowExecution,
  type AgentFlow, type FlowExecution, type NodeStatus, type FlowStatus,
} from '@/api/flows'

export const useFlowsStore = defineStore('flows', () => {
  const flows = ref<AgentFlow[]>([])
  const loading = ref(false)
  const activeFlow = ref<AgentFlow | null>(null)
  const execution = ref<FlowExecution | null>(null)
  const executionPolling = ref<ReturnType<typeof setInterval> | null>(null)

  async function refresh() {
    loading.value = true
    try {
      flows.value = await listFlows()
    } finally {
      loading.value = false
    }
  }

  async function openFlow(flow_id: string) {
    activeFlow.value = await loadFlow(flow_id)
  }

  function newFlow(): AgentFlow {
    return {
      id: '',
      name: '新工作流',
      description: '',
      nodes: [
        { id: 'start', type: 'start', label: '开始', position: { x: 100, y: 200 } },
        { id: 'end', type: 'end', label: '结束', position: { x: 600, y: 200 } },
      ],
      edges: [],
      version: 0,
      created_at: '',
      updated_at: '',
    }
  }

  async function save(flow: AgentFlow) {
    const saved = await saveFlow(flow)
    activeFlow.value = saved
    await refresh()
    return saved
  }

  async function remove(flow_id: string) {
    await deleteFlow(flow_id)
    if (activeFlow.value?.id === flow_id) activeFlow.value = null
    await refresh()
  }

  // ── 执行 ──────────────────────────────────────────────────────────────

  async function startExecution(flow: AgentFlow) {
    execution.value = await initFlowExecution(flow)
    // 轮询执行状态（每秒）
    if (executionPolling.value) clearInterval(executionPolling.value)
    executionPolling.value = setInterval(async () => {
      const exec = await getFlowExecution()
      if (exec) execution.value = exec
      if (exec?.status === 'completed' || exec?.status === 'failed') {
        stopPolling()
      }
    }, 1000)
  }

  function stopPolling() {
    if (executionPolling.value) {
      clearInterval(executionPolling.value)
      executionPolling.value = null
    }
  }

  async function markNodeRunning(node_id: string) {
    await updateNodeStatus(node_id, 'running')
  }

  async function markNodeDone(flow_id: string, node_id: string, output: unknown) {
    await setNodeOutput(flow_id, node_id, output)
    await updateNodeStatus(node_id, 'completed')
  }

  async function markNodeFailed(node_id: string, error: string) {
    await updateNodeStatus(node_id, 'failed', error)
  }

  async function log(message: string) {
    await appendFlowLog(message)
  }

  async function finish(status: FlowStatus) {
    await finishFlowExecution(status)
    stopPolling()
  }

  return {
    flows, loading, activeFlow, execution,
    refresh, openFlow, newFlow, save, remove,
    startExecution, markNodeRunning, markNodeDone, markNodeFailed, log, finish,
  }
})
