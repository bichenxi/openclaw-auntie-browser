//! Agent Flow 编排系统
//!
//! 数据传递策略：
//! - 小数据（JSON）→ 内存 FlowContext（Arc<Mutex<HashMap>>）
//! - 大数据/持久化 → ~/.openclaw/flows/<flow_id>/context/<node_id>.json

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

// ─── 数据模型 ──────────────────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FlowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String, // "agent" | "start" | "end"
    pub agent_work: Option<String>,
    pub label: String,
    pub position: NodePosition,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub condition: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AgentFlow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Paused,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NodeExecution {
    pub node_id: String,
    pub status: NodeStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Paused,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FlowExecution {
    pub id: String,
    pub flow_id: String,
    pub status: FlowStatus,
    pub node_executions: HashMap<String, NodeExecution>,
    pub logs: Vec<String>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}

// ─── 运行时状态（内存） ────────────────────────────────────────────────────

pub struct FlowRuntimeState {
    /// node_id → output data（内存传递，小数据）
    pub context: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    /// 当前执行状态
    pub execution: Arc<Mutex<Option<FlowExecution>>>,
}

impl Default for FlowRuntimeState {
    fn default() -> Self {
        Self {
            context: Arc::new(Mutex::new(HashMap::new())),
            execution: Arc::new(Mutex::new(None)),
        }
    }
}

// ─── 存储路径 ──────────────────────────────────────────────────────────────

fn flows_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let base = crate::installer::openclaw_dir(app)?;
    Ok(base.join("flows"))
}

fn flow_file(app: &AppHandle, flow_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(flows_dir(app)?.join(format!("{}.json", flow_id)))
}

fn context_dir(app: &AppHandle, flow_id: &str) -> Result<std::path::PathBuf, String> {
    Ok(flows_dir(app)?.join(flow_id).join("context"))
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_iso() -> String {
    // 简单 ISO 格式，不引入 chrono
    let ts = now_ts();
    format!("{}", ts)
}

// ─── Tauri Commands ────────────────────────────────────────────────────────

/// 列出所有已保存的 Flow
#[tauri::command]
pub fn list_flows(app: AppHandle) -> Result<Vec<AgentFlow>, String> {
    let dir = flows_dir(&app)?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut flows = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(flow) = serde_json::from_str::<AgentFlow>(&content) {
                flows.push(flow);
            }
        }
    }
    flows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(flows)
}

/// 读取单个 Flow
#[tauri::command]
pub fn load_flow(app: AppHandle, flow_id: String) -> Result<AgentFlow, String> {
    let path = flow_file(&app, &flow_id)?;
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// 保存 Flow（新建或更新）
#[tauri::command]
pub fn save_flow(app: AppHandle, mut flow: AgentFlow) -> Result<AgentFlow, String> {
    let dir = flows_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    if flow.id.is_empty() {
        flow.id = format!("flow-{}", now_ts());
        flow.created_at = now_iso();
        flow.version = 1;
    } else {
        flow.version += 1;
    }
    flow.updated_at = now_iso();

    let path = flow_file(&app, &flow.id)?;
    let content = serde_json::to_string_pretty(&flow).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(flow)
}

/// 删除 Flow
#[tauri::command]
pub fn delete_flow(app: AppHandle, flow_id: String) -> Result<(), String> {
    let path = flow_file(&app, &flow_id)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    // 清理 context 目录
    let ctx = flows_dir(&app)?.join(&flow_id);
    if ctx.exists() {
        std::fs::remove_dir_all(&ctx).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 获取当前执行状态
#[tauri::command]
pub fn get_flow_execution(
    state: tauri::State<'_, FlowRuntimeState>,
) -> Option<FlowExecution> {
    state.execution.lock().unwrap().clone()
}

/// 写入节点输出（内存 + 文件双写）
#[tauri::command]
pub fn set_node_output(
    app: AppHandle,
    state: tauri::State<'_, FlowRuntimeState>,
    flow_id: String,
    node_id: String,
    output: serde_json::Value,
) -> Result<(), String> {
    // 内存写入
    state.context.lock().unwrap().insert(node_id.clone(), output.clone());

    // 文件持久化（异步写，不阻塞）
    let ctx_dir = context_dir(&app, &flow_id)?;
    std::fs::create_dir_all(&ctx_dir).map_err(|e| e.to_string())?;
    let file = ctx_dir.join(format!("{}.json", node_id));
    let content = serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?;
    std::fs::write(file, content).map_err(|e| e.to_string())?;

    Ok(())
}

/// 读取节点输出（优先内存，fallback 文件）
#[tauri::command]
pub fn get_node_output(
    app: AppHandle,
    state: tauri::State<'_, FlowRuntimeState>,
    flow_id: String,
    node_id: String,
) -> Result<Option<serde_json::Value>, String> {
    // 先查内存
    if let Some(val) = state.context.lock().unwrap().get(&node_id).cloned() {
        return Ok(Some(val));
    }
    // fallback 文件
    let file = context_dir(&app, &flow_id)?.join(format!("{}.json", node_id));
    if file.exists() {
        let content = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let val: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(Some(val));
    }
    Ok(None)
}

/// 初始化执行（清空内存上下文，创建 FlowExecution）
#[tauri::command]
pub fn init_flow_execution(
    state: tauri::State<'_, FlowRuntimeState>,
    flow: AgentFlow,
) -> Result<FlowExecution, String> {
    // 清空内存上下文
    state.context.lock().unwrap().clear();

    let exec = FlowExecution {
        id: format!("exec-{}", now_ts()),
        flow_id: flow.id.clone(),
        status: FlowStatus::Running,
        node_executions: flow.nodes.iter().map(|n| {
            (n.id.clone(), NodeExecution {
                node_id: n.id.clone(),
                status: NodeStatus::Idle,
                output: None,
                error: None,
                started_at: None,
                finished_at: None,
            })
        }).collect(),
        logs: vec![],
        started_at: now_ts(),
        finished_at: None,
    };

    *state.execution.lock().unwrap() = Some(exec.clone());
    Ok(exec)
}

/// 更新节点执行状态
#[tauri::command]
pub fn update_node_status(
    state: tauri::State<'_, FlowRuntimeState>,
    node_id: String,
    status: NodeStatus,
    error: Option<String>,
) -> Result<(), String> {
    let mut exec_guard = state.execution.lock().unwrap();
    let exec = exec_guard.as_mut().ok_or("没有正在运行的 Flow")?;
    if let Some(node_exec) = exec.node_executions.get_mut(&node_id) {
        node_exec.status = status.clone();
        node_exec.error = error;
        match status {
            NodeStatus::Running => node_exec.started_at = Some(now_ts()),
            NodeStatus::Completed | NodeStatus::Failed => node_exec.finished_at = Some(now_ts()),
            _ => {}
        }
    }
    Ok(())
}

/// 追加执行日志
#[tauri::command]
pub fn append_flow_log(
    state: tauri::State<'_, FlowRuntimeState>,
    message: String,
) -> Result<(), String> {
    let mut exec_guard = state.execution.lock().unwrap();
    let exec = exec_guard.as_mut().ok_or("没有正在运行的 Flow")?;
    let ts = now_ts();
    exec.logs.push(format!("[{}] {}", ts, message));
    // 最多保留 1000 条
    if exec.logs.len() > 1000 {
        exec.logs.drain(0..100);
    }
    Ok(())
}

/// 结束执行
#[tauri::command]
pub fn finish_flow_execution(
    state: tauri::State<'_, FlowRuntimeState>,
    status: FlowStatus,
) -> Result<(), String> {
    let mut exec_guard = state.execution.lock().unwrap();
    if let Some(exec) = exec_guard.as_mut() {
        exec.status = status;
        exec.finished_at = Some(now_ts());
    }
    Ok(())
}
