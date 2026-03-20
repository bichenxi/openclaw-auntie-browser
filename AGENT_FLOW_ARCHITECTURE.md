# Agent Flow 编排系统 — 完整技术架构设计

**版本**: 1.0
**日期**: 2026-03-20
**状态**: 架构设计 (Phase 0 → Phase 1 规划)

---

## 一、概述

本文档设计 Oclaw 中 **Agent 协作** 与 **VueFlow 任务编排** 的完整技术体系。目标是使用户能够：
1. 通过可视化拖拽创建 Agent 工作流（DAG）
2. 定义 Agent 间的协作关系和数据传递
3. 执行工作流并监视实时进度
4. 保存/加载流程模板供复用

**核心原则**：
- 逐步演进（MVP → 完整版，2周 → 6周）
- 复用现有基础设施（Agent 文件系统、Tauri commands）
- 类型安全（TS 前端 + Rust 后端）
- 用户友好（直观的拖拽编辑）

---

## 二、数据模型设计

### 2.1 核心数据结构

#### 2.1.1 AgentFlow（工作流图）

```typescript
// src/types/agentFlow.ts

/** 工作流全局元数据 */
export interface AgentFlow {
  id: string              // UUID，唯一标识
  name: string            // 流程名称，例如 "网页采集 → 数据清洗"
  description?: string    // 流程描述
  version: string         // 版本号，语义化 (1.0.0)

  // 图结构
  nodes: FlowNode[]
  edges: FlowEdge[]

  // 元数据
  createdAt: number       // 创建时间戳
  updatedAt: number       // 更新时间戳
  author?: string         // 创建者（暂预留）
  tags?: string[]         // 标签分类
}

/** 流程节点 */
export type FlowNode = AgentNode | TaskNode | GatewayNode

/** Agent 节点（调用一个已配置的智能体） */
export interface AgentNode {
  id: string              // 节点唯一 ID (e.g., "agent-main", "agent-researcher")
  type: 'agent'
  data: {
    agentWork: string     // 引用的 Agent work 名称（e.g., "main", "work"）
    label: string         // 显示名称
    prompt?: string       // 传给 Agent 的额外 prompt（可选）
    timeout?: number      // 超时时间 (ms)
  }
  position: { x: number; y: number }

  // 执行状态（在 Flow 执行时动态更新）
  status?: NodeStatus     // 'idle' | 'running' | 'done' | 'error'
  result?: unknown        // 执行结果
  error?: string          // 错误信息
}

/** 任务节点（通用任务节点，例如数据转换、分支判断） */
export interface TaskNode {
  id: string
  type: 'task'
  data: {
    label: string
    operation: string     // 操作类型："transform" | "split" | "merge" | "wait" | ...
    config?: Record<string, unknown>  // 操作配置
  }
  position: { x: number; y: number }
  status?: NodeStatus
  result?: unknown
  error?: string
}

/** 条件网关节点（条件分支、循环等） */
export interface GatewayNode {
  id: string
  type: 'gateway'
  data: {
    label: string
    gatewayType: 'if-else' | 'switch' | 'merge'
    condition?: string    // 条件表达式或 JavaScript 代码
  }
  position: { x: number; y: number }
  status?: NodeStatus
}

/** 流程边（连接关系） */
export interface FlowEdge {
  id: string              // 边唯一 ID (e.g., "edge-1")
  source: string          // 源节点 ID
  target: string          // 目标节点 ID
  data?: {
    label?: string        // 边标签（例如条件标签 "success" / "error"）
    condition?: string    // 边上的条件（可选，用于条件路由）
  }
}

/** 节点执行状态 */
export type NodeStatus = 'idle' | 'pending' | 'running' | 'done' | 'error' | 'skipped'

/** 节点执行结果 */
export interface NodeResult {
  nodeId: string
  status: NodeStatus
  output?: unknown        // 节点输出（供下游节点使用）
  error?: string
  duration?: number       // 执行耗时 (ms)
  startTime?: number
  endTime?: number
}
```

#### 2.1.2 流程执行状态

```typescript
// src/types/agentFlowExecution.ts

/** 流程执行实例 */
export interface FlowExecution {
  id: string              // 执行实例 ID (UUID)
  flowId: string          // 关联的 AgentFlow ID
  status: ExecutionStatus // 全局执行状态

  // 执行时间
  startTime?: number
  endTime?: number
  duration?: number       // 总耗时 (ms)

  // 节点执行结果映射
  nodeResults: Map<string, NodeResult>

  // 中断/恢复信息
  pausedAt?: number       // 暂停时间戳
  resumeFrom?: string     // 从某个节点恢复执行

  // 日志
  logs: ExecutionLog[]
}

/** 执行状态 */
export type ExecutionStatus = 'pending' | 'running' | 'paused' | 'done' | 'error' | 'cancelled'

/** 执行日志条目 */
export interface ExecutionLog {
  timestamp: number
  nodeId?: string
  level: 'info' | 'warn' | 'error'
  message: string
}
```

#### 2.1.3 与现有 Agent 模型的关系

```
AgentInfo (已有)
├─ name: string          // Agent 显示名称 ("main", "researcher", ...)
├─ workspace: string     // 对应的 workspace 目录
└─ description?: string

       ↓ (在 AgentFlow 中引用)

AgentNode (新增)
├─ agentWork: string     // 引用 AgentInfo.workspace
└─ data.label: string    // 节点在流程中的显示名
```

### 2.2 存储结构

#### 2.2.1 文件系统组织

```
~/.openclaw/
├── workspace/           # 默认 Agent
├── workspace-main/      # Agent "main"
├── workspace-researcher/
└── flows/               # 新增：工作流存储目录
    ├── flow-1.json      # AgentFlow JSON
    ├── flow-2.json
    └── flow-executions/ # 执行历史
        ├── exec-1.json  # FlowExecution JSON
        └── exec-2.json
```

#### 2.2.2 AgentFlow JSON Schema

```json
{
  "id": "flow-abc123",
  "name": "网页采集与数据清洗",
  "description": "从网页获取数据，经过数据清洗后输出",
  "version": "1.0.0",
  "createdAt": 1711000000000,
  "updatedAt": 1711000000000,
  "nodes": [
    {
      "id": "agent-collector",
      "type": "agent",
      "data": {
        "agentWork": "collector",
        "label": "网页采集 Agent",
        "prompt": "从指定URL采集结构化数据",
        "timeout": 30000
      },
      "position": { "x": 100, "y": 100 }
    },
    {
      "id": "agent-cleaner",
      "type": "agent",
      "data": {
        "agentWork": "cleaner",
        "label": "数据清洗 Agent"
      },
      "position": { "x": 350, "y": 100 }
    }
  ],
  "edges": [
    {
      "id": "edge-1",
      "source": "agent-collector",
      "target": "agent-cleaner",
      "data": { "label": "采集结果" }
    }
  ]
}
```

---

## 三、前端架构设计

### 3.1 新增 Vue 组件

```
src/
├── components/
│   ├── AgentFlowEditor.vue        # Flow 编辑器主容器
│   ├── AgentFlowCanvas.vue        # VueFlow 画布
│   ├── nodes/
│   │   ├── AgentFlowNode.vue      # Agent 节点自定义渲染
│   │   ├── TaskFlowNode.vue       # Task 节点自定义渲染
│   │   └── GatewayFlowNode.vue    # Gateway 节点自定义渲染
│   ├── FlowToolbar.vue            # 工具栏（保存、执行、导出等）
│   ├── FlowProperties.vue         # 右侧属性面板
│   ├── FlowExecutionMonitor.vue   # 执行监视窗口
│   └── FlowNodeInspector.vue      # 节点检查器（右键菜单）
├── views/
│   └── AgentFlowPage.vue          # Flow 编辑页面（新增 route）
└── stores/
    └── agentFlows.ts             # Pinia store
```

### 3.2 Pinia Store 扩展

```typescript
// src/stores/agentFlows.ts

import { defineStore } from 'pinia'
import type { AgentFlow, FlowExecution, NodeStatus } from '@/types/agentFlow'

export const useAgentFlowsStore = defineStore('agentFlows', () => {
  // ─── 工作流列表 ────────────────────────────────────
  const flows = ref<AgentFlow[]>([])
  const currentFlowId = ref<string | null>(null)

  const currentFlow = computed(() =>
    currentFlowId.value
      ? flows.value.find(f => f.id === currentFlowId.value)
      : null
  )

  // ─── 编辑状态 ────────────────────────────────────
  const isDirty = ref(false)  // 有未保存修改
  const isEditing = ref(false) // 进入编辑模式

  // ─── 执行状态 ────────────────────────────────────
  const currentExecution = ref<FlowExecution | null>(null)
  const executionHistory = ref<FlowExecution[]>([])
  const isExecuting = ref(false)

  // ─── 节点选中 ────────────────────────────────────
  const selectedNodeId = ref<string | null>(null)
  const selectedNodeData = computed(() =>
    currentFlow.value?.nodes.find(n => n.id === selectedNodeId.value)
  )

  // ─── Actions ────────────────────────────────────
  async function loadFlows() {
    flows.value = await invoke<AgentFlow[]>('list_agent_flows')
  }

  async function createFlow(name: string, description?: string) {
    const flow: AgentFlow = {
      id: `flow-${Date.now()}`,
      name,
      description,
      version: '1.0.0',
      nodes: [],
      edges: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    }
    flows.value.push(flow)
    currentFlowId.value = flow.id
    isDirty.value = true
    return flow
  }

  async function saveFlow(flow?: AgentFlow) {
    const toSave = flow || currentFlow.value
    if (!toSave) throw new Error('No flow to save')

    toSave.updatedAt = Date.now()
    await invoke('save_agent_flow', { flow: toSave })
    isDirty.value = false
  }

  async function deleteFlow(flowId: string) {
    const idx = flows.value.findIndex(f => f.id === flowId)
    if (idx !== -1) flows.value.splice(idx, 1)
    await invoke('delete_agent_flow', { flowId })
    if (currentFlowId.value === flowId) currentFlowId.value = null
  }

  // ─── 节点操作 ────────────────────────────────────
  function addNode(node: FlowNode) {
    if (!currentFlow.value) return
    currentFlow.value.nodes.push(node)
    isDirty.value = true
  }

  function removeNode(nodeId: string) {
    if (!currentFlow.value) return
    currentFlow.value.nodes = currentFlow.value.nodes.filter(n => n.id !== nodeId)
    currentFlow.value.edges = currentFlow.value.edges.filter(
      e => e.source !== nodeId && e.target !== nodeId
    )
    isDirty.value = true
  }

  function updateNodeData(nodeId: string, data: Record<string, unknown>) {
    if (!currentFlow.value) return
    const node = currentFlow.value.nodes.find(n => n.id === nodeId)
    if (node) {
      Object.assign(node.data, data)
      isDirty.value = true
    }
  }

  function updateNodePosition(nodeId: string, position: { x: number; y: number }) {
    if (!currentFlow.value) return
    const node = currentFlow.value.nodes.find(n => n.id === nodeId)
    if (node) {
      node.position = position
    }
  }

  // ─── 边操作 ────────────────────────────────────
  function addEdge(edge: FlowEdge) {
    if (!currentFlow.value) return
    currentFlow.value.edges.push(edge)
    isDirty.value = true
  }

  function removeEdge(edgeId: string) {
    if (!currentFlow.value) return
    currentFlow.value.edges = currentFlow.value.edges.filter(e => e.id !== edgeId)
    isDirty.value = true
  }

  // ─── 执行控制 ────────────────────────────────────
  async function runFlow(flowId?: string, params?: Record<string, unknown>) {
    const flow = flowId
      ? flows.value.find(f => f.id === flowId)
      : currentFlow.value
    if (!flow) throw new Error('Flow not found')

    isExecuting.value = true
    try {
      const execution = await invoke<FlowExecution>('run_agent_flow', {
        flowId: flow.id,
        params
      })
      currentExecution.value = execution
      executionHistory.value.unshift(execution)
      return execution
    } finally {
      isExecuting.value = false
    }
  }

  async function cancelExecution() {
    if (!currentExecution.value) return
    await invoke('cancel_agent_flow_execution', {
      executionId: currentExecution.value.id
    })
  }

  async function getExecutionStatus(executionId: string): Promise<FlowExecution> {
    return invoke('get_agent_flow_execution_status', { executionId })
  }

  return {
    // State
    flows,
    currentFlowId,
    currentFlow,
    isDirty,
    isEditing,
    currentExecution,
    executionHistory,
    isExecuting,
    selectedNodeId,
    selectedNodeData,

    // Actions
    loadFlows,
    createFlow,
    saveFlow,
    deleteFlow,
    addNode,
    removeNode,
    updateNodeData,
    updateNodePosition,
    addEdge,
    removeEdge,
    runFlow,
    cancelExecution,
    getExecutionStatus,
  }
})
```

### 3.3 VueFlow 集成配置

```typescript
// src/composables/useFlowEditor.ts

import { useVueFlow, useEdges, useNodes } from '@vue-flow/core'
import type { FlowNode, FlowEdge } from '@/types/agentFlow'

export function useFlowEditor() {
  const store = useAgentFlowsStore()
  const { addNodes, addEdges, getNodes, getEdges } = useVueFlow()

  // ─── VueFlow 节点类型定义 ────────────────────────
  const nodeTypes = {
    agent: defineAsyncComponent(() => import('@/components/nodes/AgentFlowNode.vue')),
    task: defineAsyncComponent(() => import('@/components/nodes/TaskFlowNode.vue')),
    gateway: defineAsyncComponent(() => import('@/components/nodes/GatewayFlowNode.vue')),
  }

  // ─── 初始化工作流 ────────────────────────────────
  function initializeFlow(flow: AgentFlow) {
    // 转换内部模型 → VueFlow 模型
    const vueFlowNodes = flow.nodes.map(node => ({
      id: node.id,
      label: node.data.label,
      position: node.position,
      type: node.type,
      data: node.data,
      status: node.status || 'idle',
    }))

    const vueFlowEdges = flow.edges.map(edge => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      label: edge.data?.label,
    }))

    addNodes(vueFlowNodes)
    addEdges(vueFlowEdges)
  }

  // ─── 同步到 Store ────────────────────────────────
  function syncToStore() {
    if (!store.currentFlow) return

    const nodes = getNodes.value
    const edges = getEdges.value

    store.currentFlow.nodes = nodes.map(n => ({
      id: n.id,
      type: n.type as any,
      data: n.data,
      position: n.position,
      status: (n as any).status || 'idle',
    }))

    store.currentFlow.edges = edges.map(e => ({
      id: e.id,
      source: e.source,
      target: e.target,
      data: { label: e.label },
    }))

    store.isDirty = true
  }

  return {
    nodeTypes,
    initializeFlow,
    syncToStore,
  }
}
```

### 3.4 路由注册

```typescript
// src/pages/agent-flows.vue（自动由 unplugin-vue-router 注册）

<template>
  <AgentFlowPage />
</template>
```

在 `src/stores/tabs.ts` 中扩展 `SpecialView`:

```typescript
export type SpecialView = 'openclaw' | 'settings' | 'skills' | 'setup' | 'agents' | 'agent-editor' | 'agent-flows'
```

---

## 四、后端架构设计（Rust/Tauri）

### 4.1 新增 Module

```rust
// src-tauri/src/lib.rs

mod agent_flows;  // 新增模块

// 在 generate_handler! 中注册新命令
.invoke_handler(generate_handler![
    // ... 现有命令 ...
    agent_flows::list_agent_flows,
    agent_flows::create_agent_flow,
    agent_flows::get_agent_flow,
    agent_flows::save_agent_flow,
    agent_flows::delete_agent_flow,
    agent_flows::run_agent_flow,
    agent_flows::get_agent_flow_execution_status,
    agent_flows::cancel_agent_flow_execution,
])
```

### 4.2 Agent Flows 模块实现

```rust
// src-tauri/src/agent_flows.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;

// ─── 数据模型 ────────────────────────────────────
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AgentFlow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum FlowNode {
    #[serde(rename = "agent")]
    Agent { id: String, data: AgentNodeData, position: Position },
    #[serde(rename = "task")]
    Task { id: String, data: TaskNodeData, position: Position },
    #[serde(rename = "gateway")]
    Gateway { id: String, data: GatewayNodeData, position: Position },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AgentNodeData {
    pub agent_work: String,
    pub label: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskNodeData {
    pub label: String,
    pub operation: String,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GatewayNodeData {
    pub label: String,
    pub gateway_type: String,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Position { pub x: f64, pub y: f64 }

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub data: Option<EdgeData>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EdgeData {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub condition: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FlowExecution {
    pub id: String,
    pub flow_id: String,
    pub status: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub duration: Option<u64>,
    pub node_results: HashMap<String, NodeResult>,
    pub logs: Vec<ExecutionLog>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NodeResult {
    pub node_id: String,
    pub status: String,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecutionLog {
    pub timestamp: i64,
    #[serde(default)]
    pub node_id: Option<String>,
    pub level: String,
    pub message: String,
}

// ─── 命令处理 ────────────────────────────────────

#[tauri::command]
pub fn list_agent_flows(app: AppHandle) -> Result<Vec<AgentFlow>, String> {
    let flows_dir = flows_directory(&app)?;
    if !flows_dir.exists() {
        return Ok(vec![]);
    }

    let mut flows = Vec::new();
    for entry in std::fs::read_dir(&flows_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.extension().and_then(|s| s.to_str()).map_or(false, |s| s == "json") {
            continue;
        }

        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let flow: AgentFlow = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        flows.push(flow);
    }

    flows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(flows)
}

#[tauri::command]
pub fn get_agent_flow(app: AppHandle, flow_id: String) -> Result<AgentFlow, String> {
    let path = flows_directory(&app)?.join(format!("{}.json", flow_id));
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_agent_flow(app: AppHandle, flow: AgentFlow) -> Result<(), String> {
    let flows_dir = flows_directory(&app)?;
    std::fs::create_dir_all(&flows_dir).map_err(|e| e.to_string())?;

    let path = flows_dir.join(format!("{}.json", flow.id));
    let content = serde_json::to_string_pretty(&flow).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_agent_flow(app: AppHandle, flow_id: String) -> Result<(), String> {
    let path = flows_directory(&app)?.join(format!("{}.json", flow_id));
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_agent_flow(
    app: AppHandle,
    flow_id: String,
    params: Option<serde_json::Value>,
) -> Result<FlowExecution, String> {
    // TODO: 实现流程执行引擎
    // 1. 拓扑排序节点
    // 2. DAG 执行调度
    // 3. Agent 间数据传递

    let execution_id = format!("exec-{}", uuid::Uuid::new_v4());
    let execution = FlowExecution {
        id: execution_id,
        flow_id,
        status: "running".to_string(),
        start_time: Some(chrono::Local::now().timestamp_millis()),
        end_time: None,
        duration: None,
        node_results: HashMap::new(),
        logs: vec![],
    };

    Ok(execution)
}

#[tauri::command]
pub fn get_agent_flow_execution_status(
    app: AppHandle,
    execution_id: String,
) -> Result<FlowExecution, String> {
    // TODO: 从存储中读取执行状态
    Err("Not implemented".to_string())
}

#[tauri::command]
pub fn cancel_agent_flow_execution(
    app: AppHandle,
    execution_id: String,
) -> Result<(), String> {
    // TODO: 取消执行
    Ok(())
}

// ─── 工具函数 ────────────────────────────────────

fn flows_directory(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let openclaw_dir = crate::installer::openclaw_dir(app)?;
    Ok(openclaw_dir.join("flows"))
}
```

### 4.3 Cargo.toml 依赖更新

```toml
[dependencies]
# ... 现有依赖 ...
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
# DAG 拓扑排序库（Phase 2）
petgraph = "0.6"
```

---

## 五、Agent 间协作执行引擎设计

### 5.1 执行流程设计

```
┌─────────────────────────────────────────────────────┐
│ 开始执行 Flow                                        │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 1. 拓扑排序 DAG 节点                                │
│    - 使用 Kahn 算法或 DFS 确定执行顺序             │
│    - 检测环（不允许）                               │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 2. 初始化执行上下文 (ExecutionContext)             │
│    - context.nodeOutputs: Map<nodeId, output>     │
│    - context.params: 输入参数                       │
│    - context.logs: 执行日志                         │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 3. 执行队列中的节点                                │
│    循环直到队列为空：                               │
│    - 检查前置节点是否完成                           │
│    - 是 → 执行节点，更新 context                   │
│    - 否 → 继续等待                                  │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 4. 节点执行处理                                    │
│    根据节点类型：                                   │
│    - AgentNode: 调用 Agent (via OpenClaw CLI)     │
│    - TaskNode: 执行本地操作（transform/merge）    │
│    - GatewayNode: 计算条件，决定分支               │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 5. 数据传递机制                                    │
│    output[nodeA] → input for nodeB                 │
│    - 通过边标签和条件判断路由                       │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 6. 错误处理                                        │
│    - 节点失败 → 标记为 error                      │
│    - 是否继续？（依赖策略）                         │
│    - 记录错误日志                                   │
└────────────────────┬────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────────┐
│ 7. 完成执行                                        │
│    - 更新总体状态                                   │
│    - 保存执行记录                                   │
│    - 返回最终结果                                   │
└─────────────────────────────────────────────────────┘
```

### 5.2 Pseudo Code（阶段 2 实现）

```rust
// 伪代码，具体实现在 Phase 2

pub struct ExecutionEngine {
    context: ExecutionContext,
    flow: AgentFlow,
}

impl ExecutionEngine {
    pub async fn run(&mut self) -> Result<FlowExecution, String> {
        // 1. 拓扑排序
        let sorted_nodes = self.topological_sort()?;

        // 2. 初始化
        self.context.start_time = now();

        // 3. 执行
        for node_id in sorted_nodes {
            let node = self.flow.nodes.iter().find(|n| n.id == node_id)?;

            // 检查前置条件
            if !self.check_predecessors_done(node_id) {
                continue;
            }

            // 执行节点
            let result = match node {
                FlowNode::Agent { data, .. } => {
                    self.execute_agent_node(data).await
                }
                FlowNode::Task { data, .. } => {
                    self.execute_task_node(data)
                }
                FlowNode::Gateway { data, .. } => {
                    self.execute_gateway_node(data)
                }
            };

            // 保存结果
            self.context.node_results.insert(node_id.clone(), result);

            // 更新日志
            self.context.logs.push(ExecutionLog { /* ... */ });
        }

        // 4. 返回
        self.build_execution()
    }

    async fn execute_agent_node(&mut self, data: &AgentNodeData) -> NodeResult {
        // 调用 OpenClaw Agent
        // 传递 input data（来自前置节点）
        // 等待完成并收集 output
    }

    fn execute_task_node(&mut self, data: &TaskNodeData) -> NodeResult {
        // 根据 operation 类型执行本地操作
        match data.operation.as_str() {
            "transform" => { /* 数据转换 */ }
            "merge" => { /* 合并 */ }
            "split" => { /* 分割 */ }
            _ => NodeResult { status: "error".to_string(), ..default() }
        }
    }

    fn execute_gateway_node(&mut self, data: &GatewayNodeData) -> NodeResult {
        // 评估条件表达式
        // 返回路由结果
    }
}
```

### 5.3 数据传递机制

```typescript
// 节点间数据流示例

AgentFlow {
  nodes: [
    {
      id: "agent-collector",
      type: "agent",
      data: { agentWork: "collector" }
    },
    {
      id: "task-transform",
      type: "task",
      data: {
        operation: "transform",
        config: {
          // 使用占位符引用前置节点输出
          input: "{{ agent-collector.output }}"
        }
      }
    }
  ]
}

// 执行时，context 会保存：
context.nodeOutputs = {
  "agent-collector": {
    data: [...],
    status: "done"
  },
  "task-transform": {
    data: [...],
    status: "done"
  }
}
```

---

## 六、渐进式实施路线图

### Phase 1：MVP（2周，可交付 Week 1-2）

**目标**：可视化编辑 + 基础执行

#### 前端
- [x] AgentFlow 数据模型定义
- [ ] AgentFlowPage 页面框架
- [ ] VueFlow 集成（基础画布）
- [ ] 手工添加节点（暂无拖拽）
- [ ] 保存/加载流程到本地

#### 后端
- [ ] `list_agent_flows` 命令
- [ ] `save_agent_flow` 命令
- [ ] `get_agent_flow` 命令
- [ ] 流程文件存储（`~/.openclaw/flows/`）

#### 不需要
- 实际执行引擎
- 条件分支
- 数据传递

**输出物**：
- 用户可以创建、编辑、保存工作流
- UI 显示节点和边的拓扑结构

---

### Phase 2：单 Agent 执行（1周，Week 3）

**目标**：运行单个 Agent 节点的简单流程

#### 前端
- [ ] 运行按钮 & 执行监视窗口
- [ ] 节点状态展示（running/done/error）
- [ ] 实时日志查看
- [ ] 停止/取消执行

#### 后端
- [ ] `run_agent_flow` 命令（单 Agent 路径）
- [ ] `get_agent_flow_execution_status` 命令
- [ ] `cancel_agent_flow_execution` 命令
- [ ] 执行状态存储

#### 实现
- DAG 检验（无环）
- 顺序执行单个 Agent 节点
- 基础日志记录

---

### Phase 3：Agent 协作 & 数据传递（2周，Week 4-5）

**目标**：多 Agent 工作流 + 数据流通

#### 前端
- [ ] 边上的数据标签和条件编辑
- [ ] 占位符语法提示（`{{ nodeId.output }}`）
- [ ] 节点间数据连线可视化

#### 后端
- [ ] 执行引擎完整实现（Phase 5.2 中的伪代码）
- [ ] 拓扑排序 + 并行调度
- [ ] 上下文数据传递（context.nodeOutputs）
- [ ] Agent 通信协议（传递 input data）

#### 数据流
```
Agent A (collector)
  ↓ output: { data: [...] }
Agent B (cleaner) ← input: {{ collector.output }}
  ↓ output: { cleaned: [...] }
Agent C (exporter) ← input: {{ cleaner.output }}
```

---

### Phase 4：条件 & 分支（1周，Week 6）

**目标**：支持 if-else、switch 条件流程

#### 前端
- [ ] GatewayNode 编辑器
- [ ] 条件表达式编辑
- [ ] 分支标签显示

#### 后端
- [ ] Gateway 节点执行
- [ ] 条件表达式求值（简单 JS eval）
- [ ] 动态路由

---

### Phase 5：高级功能（可选，Week 7+）

- [ ] 循环/重试节点
- [ ] 人机混合暂停点
- [ ] 流程版本管理
- [ ] 团队协作（共享模板）
- [ ] AI 自动生成工作流

---

## 七、风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| DAG 循环检测复杂 | Phase 3 卡壳 | 使用开源库 `petgraph` |
| Agent 通信超时 | 流程卡死 | 设置全局 timeout + 强制中止 |
| 大规模流程性能 | 编辑/执行卡顿 | 限制节点数 (e.g., 100)，异步执行 |
| 条件表达式安全 | 代码注入 | 白名单操作符，禁止 eval |
| 用户学习曲线 | 采用率低 | 提供示例模板，交互式教程 |

---

## 八、API 快速参考（Tauri Commands）

### Frontend API

```typescript
// 流程管理
invoke<AgentFlow[]>('list_agent_flows')
invoke<AgentFlow>('get_agent_flow', { flowId })
invoke('save_agent_flow', { flow })
invoke('delete_agent_flow', { flowId })

// 执行控制
invoke<FlowExecution>('run_agent_flow', { flowId, params? })
invoke<FlowExecution>('get_agent_flow_execution_status', { executionId })
invoke('cancel_agent_flow_execution', { executionId })
```

---

## 九、关键决策文档

### 9.1 为什么用 VueFlow？

- ✅ 轻量级（相比 Mermaid）
- ✅ 支持自定义节点类型
- ✅ 响应式数据绑定
- ✅ 活跃社区

### 9.2 为什么存储在 `~/.openclaw/flows/`？

- 与现有 Agent 目录结构一致
- 用户可通过 git 版本控制
- 跨平台兼容

### 9.3 为什么分 Phase？

- MVP 快速验证可行性（2周）
- Phase 2/3 逐步完善（3周）
- 降低技术风险，及时调整

---

## 十、总结

本架构设计提供了 **完整的 Agent 协作编排系统** 框架：

1. **清晰的数据模型**：AgentFlow = 图 + 节点 + 边 + 执行状态
2. **渐进式实施**：从 MVP 到完整版，风险可控
3. **前后端分离**：TypeScript + Rust，类型安全
4. **可扩展设计**：为高级功能预留接口

**下一步行动**：
- Phase 1 开发排期：约 2 周
- 优先完成：数据模型 + 基础 UI + 文件存储
- 后续根据用户反馈迭代
