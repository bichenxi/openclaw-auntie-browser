import { invoke } from '@tauri-apps/api/core'

export interface NodePosition {
  x: number
  y: number
}

export interface FlowNode {
  id: string
  type: 'agent' | 'start' | 'end'
  agent_work?: string
  label: string
  position: NodePosition
}

export interface FlowEdge {
  id: string
  source: string
  target: string
  condition?: string
}

export interface AgentFlow {
  id: string
  name: string
  description?: string
  nodes: FlowNode[]
  edges: FlowEdge[]
  version: number
  created_at: string
  updated_at: string
}

export type NodeStatus = 'idle' | 'running' | 'completed' | 'failed' | 'paused'
export type FlowStatus = 'idle' | 'running' | 'completed' | 'failed' | 'paused'

export interface NodeExecution {
  node_id: string
  status: NodeStatus
  output?: unknown
  error?: string
  started_at?: number
  finished_at?: number
}

export interface FlowExecution {
  id: string
  flow_id: string
  status: FlowStatus
  node_executions: Record<string, NodeExecution>
  logs: string[]
  started_at: number
  finished_at?: number
}

export const listFlows = () => invoke<AgentFlow[]>('list_flows')
export const loadFlow = (flow_id: string) => invoke<AgentFlow>('load_flow', { flow_id })
export const saveFlow = (flow: AgentFlow) => invoke<AgentFlow>('save_flow', { flow })
export const deleteFlow = (flow_id: string) => invoke<void>('delete_flow', { flow_id })

export const getFlowExecution = () => invoke<FlowExecution | null>('get_flow_execution')
export const initFlowExecution = (flow: AgentFlow) => invoke<FlowExecution>('init_flow_execution', { flow })
export const updateNodeStatus = (node_id: string, status: NodeStatus, error?: string) =>
  invoke<void>('update_node_status', { node_id, status, error: error ?? null })
export const setNodeOutput = (flow_id: string, node_id: string, output: unknown) =>
  invoke<void>('set_node_output', { flow_id, node_id, output })
export const getNodeOutput = (flow_id: string, node_id: string) =>
  invoke<unknown | null>('get_node_output', { flow_id, node_id })
export const appendFlowLog = (message: string) => invoke<void>('append_flow_log', { message })
export const finishFlowExecution = (status: FlowStatus) =>
  invoke<void>('finish_flow_execution', { status })
