# Agent Flow System — Phase 1 实施清单

**目标**：可视化编辑 + 基础文件持久化（不包含执行引擎）
**预计周期**：2 周
**优先级**：高（为 Phase 2/3 奠定基础）

---

## 任务分解

### 前端任务

#### 1. 数据模型与类型定义（1 天）

- [ ] 创建 `src/types/agentFlow.ts`
  - [ ] `AgentFlow` 接口
  - [ ] `FlowNode` / `AgentNode` / `TaskNode` / `GatewayNode`
  - [ ] `FlowEdge` 接口
  - [ ] `NodeStatus` 枚举

- [ ] 创建 `src/types/agentFlowExecution.ts`
  - [ ] `FlowExecution` 接口
  - [ ] `NodeResult` 接口
  - [ ] `ExecutionLog` 接口

**验收标准**：
- [ ] TS 编译无错
- [ ] 所有类型在单测中可正确引用

---

#### 2. Pinia Store 实现（1 天）

- [ ] 创建 `src/stores/agentFlows.ts`

**包含功能**：
- [ ] 流程列表管理（loadFlows, createFlow, deleteFlow）
- [ ] 当前流程编辑状态 (currentFlowId, isDirty)
- [ ] 节点和边的增删改（addNode, removeNode, updateNodeData）
- [ ] 选中状态（selectedNodeId）

**验收标准**：
- [ ] Store 可正常加载
- [ ] Actions 支持所有基础操作
- [ ] Ref 和 Computed 正确响应

---

#### 3. VueFlow 集成与画布（1.5 天）

- [ ] 安装 VueFlow 依赖
  ```bash
  npm install @vue-flow/core @vue-flow/controls @vue-flow/minimap
  ```

- [ ] 创建 `src/composables/useFlowEditor.ts`
  - [ ] `initializeFlow()` - 将 AgentFlow 转换为 VueFlow 格式
  - [ ] `syncToStore()` - 将 VueFlow 变更同步回 Store
  - [ ] `nodeTypes` 对象映射

- [ ] 创建 `src/components/AgentFlowCanvas.vue`
  - [ ] VueFlow 容器
  - [ ] 基础画布（拖拽、缩放、平移）
  - [ ] 节点类型注册
  - [ ] 边渲染

**验收标准**：
- [ ] 画布加载无错
- [ ] 可显示节点和边
- [ ] 节点可拖拽移动
- [ ] 缩放和平移正常

---

#### 4. 节点自定义渲染（1.5 天）

- [ ] 创建 `src/components/nodes/AgentFlowNode.vue`
  - [ ] 显示 Agent 图标 + 标签
  - [ ] 显示状态指示器（运行中/完成/错误）
  - [ ] 输入输出端口

- [ ] 创建 `src/components/nodes/TaskFlowNode.vue`
  - [ ] 显示任务类型
  - [ ] 操作标签

- [ ] 创建 `src/components/nodes/GatewayFlowNode.vue`
  - [ ] 条件网关符号
  - [ ] 多出口显示（Phase 2）

**验收标准**：
- [ ] 节点UI 清晰美观（遵循 UnoCSS 规范）
- [ ] 不同节点类型视觉可区分
- [ ] 鼠标悬停显示tooltip

---

#### 5. 右侧属性面板（1 天）

- [ ] 创建 `src/components/FlowProperties.vue`
  - [ ] 显示选中节点详情
  - [ ] 编辑节点名称
  - [ ] 编辑 Agent 选择（下拉列表）
  - [ ] 编辑 Prompt（可选文本框）
  - [ ] 编辑 Timeout（数字输入）

**验收标准**：
- [ ] 选中节点时显示属性面板
- [ ] 修改属性实时更新
- [ ] 点空处时面板隐藏

---

#### 6. 工具栏（1 天）

- [ ] 创建 `src/components/FlowToolbar.vue`
  - [ ] 流程名称显示/编辑
  - [ ] 保存按钮（Save Flow）
  - [ ] 新增节点按钮（+Agent、+Task、+Gateway）
  - [ ] 删除选中节点按钮
  - [ ] 撤销/重做（可选，Phase 2）

**验收标准**：
- [ ] 按钮功能完整
- [ ] Save 按钮在有修改时亮起

---

#### 7. 页面集成（1 天）

- [ ] 创建 `src/views/AgentFlowPage.vue`（或 `src/pages/agent-flows.vue`）
  - [ ] 三栏布局：工具栏 + 画布 + 属性面板
  - [ ] 容器样式（全屏、响应式）

- [ ] 在 `src/stores/tabs.ts` 中注册 'agent-flows' SpecialView

- [ ] 在导航中添加入口
  - [ ] TabBar 或菜单中添加 "工作流编排" 链接

**验收标准**：
- [ ] 页面可正常加载
- [ ] 三个区域布局合理
- [ ] 响应式适配不同屏幕

---

### 后端任务

#### 8. Agent Flows 模块结构（1 天）

- [ ] 创建 `src-tauri/src/agent_flows.rs`

**包含内容**：
- [ ] `AgentFlow` / `FlowNode` / `FlowEdge` 数据结构（Serialize/Deserialize）
- [ ] `flows_directory()` 工具函数
- [ ] 基础错误处理

**验收标准**：
- [ ] 模块 compile 无错
- [ ] 结构与 TS 类型定义一致

---

#### 9. Tauri Commands（1 天）

在 `src-tauri/src/agent_flows.rs` 中实现：

- [ ] `list_agent_flows(app: AppHandle) -> Result<Vec<AgentFlow>, String>`
  - 扫描 `~/.openclaw/flows/` 目录
  - 按 updatedAt 倒序排列

- [ ] `get_agent_flow(app: AppHandle, flow_id: String) -> Result<AgentFlow, String>`
  - 读取指定流程文件

- [ ] `save_agent_flow(app: AppHandle, flow: AgentFlow) -> Result<(), String>`
  - 创建 `~/.openclaw/flows/` 若不存在
  - 写入 JSON 文件（pretty print）

- [ ] `delete_agent_flow(app: AppHandle, flow_id: String) -> Result<(), String>`
  - 删除流程文件

**验收标准**：
- [ ] 所有 commands 在 `lib.rs` 中注册
- [ ] Tauri 编译成功
- [ ] Commands 可被前端正确调用

---

#### 10. 文件存储（1 天）

- [ ] 流程文件位置：`~/.openclaw/flows/`
  - [ ] 文件名格式：`{flow-id}.json`
  - [ ] JSON 结构与 API Schema 对齐

- [ ] 添加 Cargo.toml 依赖（如需）
  - [ ] `serde` 和 `serde_json` 已有

**验收标准**：
- [ ] 创建的流程可被保存到磁盘
- [ ] 保存的 JSON 结构正确
- [ ] 可重新加载并反序列化

---

### 前端 API 层（0.5 天）

- [ ] 创建 `src/api/agentFlows.ts`

```typescript
export async function listAgentFlows(): Promise<AgentFlow[]>
export async function getAgentFlow(flowId: string): Promise<AgentFlow>
export async function saveAgentFlow(flow: AgentFlow): Promise<void>
export async function deleteAgentFlow(flowId: string): Promise<void>
```

**验收标准**：
- [ ] 所有 API 对应 Tauri Commands
- [ ] 参数和返回值与后端一致

---

## 集成测试

### 端到端流程

- [ ] 打开应用 → 导航到 "工作流编排"
- [ ] 点击 "新建流程"
  - [ ] 输入流程名称
  - [ ] 流程在列表中出现 ✓
- [ ] 添加 Agent 节点
  - [ ] 点击 "新增节点 → Agent"
  - [ ] 节点在画布上出现 ✓
  - [ ] 可拖拽移动 ✓
- [ ] 编辑节点
  - [ ] 点击节点
  - [ ] 右侧属性面板显示 ✓
  - [ ] 修改 Agent 选择
  - [ ] 修改 Prompt ✓
- [ ] 保存流程
  - [ ] 点击 Save 按钮
  - [ ] 文件写入 `~/.openclaw/flows/` ✓
  - [ ] "已保存" 提示出现 ✓
- [ ] 重新加载
  - [ ] 关闭并重开应用
  - [ ] 流程列表中显示已保存流程 ✓
  - [ ] 节点和编辑内容恢复 ✓

---

## 验收标准（Phase 1 完成）

### 功能完整性
- [ ] 可创建新工作流
- [ ] 可可视化编辑节点和边
- [ ] 可保存到本地文件
- [ ] 可加载已保存的流程
- [ ] 可删除流程

### UI/UX
- [ ] 界面美观（遵循 UnoCSS 原子类）
- [ ] 响应流畅（无明显卡顿）
- [ ] 提示信息清晰（保存成功、错误提示）

### 技术质量
- [ ] TypeScript 编译无警告
- [ ] Tauri 编译成功
- [ ] 无运行时异常

### 文档
- [ ] 代码注释完整
- [ ] Store actions 有 JSDoc
- [ ] Tauri commands 有文档字符串

---

## 其他注意事项

### VueFlow 版本选择
推荐使用最新 `@vue-flow/core` v1.x（支持 Vue 3.5）

### 样式标准
- 优先使用 UnoCSS 原子类（参考 `unocss.config.ts`）
- 节点高度：40-50px，宽度：120-150px
- 背景色：使用 secondary/primary 主题色
- Hover 动画：使用 `transition` 快捷类

### 命名规范
- Vue 组件：`PascalCase` (AgentFlowNode.vue)
- Store actions：`camelCase` (addNode, removeEdge)
- Tauri commands：`snake_case` (list_agent_flows)

### 错误处理
- 所有 async 操作需要 try-catch
- 向用户显示友好的错误提示（ElMessage）
- 后台记录详细错误日志

---

## 里程碑

| 任务 | 估算 | 依赖 | 状态 |
|------|------|------|------|
| 类型定义 | 1天 | - | ⬜ |
| Pinia Store | 1天 | 类型 | ⬜ |
| VueFlow 集成 | 1.5天 | 类型、Store | ⬜ |
| 节点组件 | 1.5天 | VueFlow | ⬜ |
| 属性面板 | 1天 | Store | ⬜ |
| 工具栏 | 1天 | Store | ⬜ |
| 页面集成 | 1天 | 所有前端 | ⬜ |
| **前端合计** | **8天** | | |
| Rust 模块 | 1天 | - | ⬜ |
| Tauri Commands | 1天 | 模块 | ⬜ |
| 文件存储 | 1天 | Commands | ⬜ |
| 前端 API | 0.5天 | Commands | ⬜ |
| **后端合计** | **3.5天** | | |
| **总计** | **10天** | | |

*实际工期可能因现有代码复用而减少*

---

## 审查检查清单

完成时请确认：

- [ ] 所有 TS 类型定义完整且编译无错
- [ ] Pinia Store 的 actions 都有实现
- [ ] VueFlow 画布加载无黑屏
- [ ] 所有节点类型都有对应的 Vue 组件
- [ ] 保存流程后文件确实存在于 `~/.openclaw/flows/`
- [ ] 加载已保存流程，内容完全恢复
- [ ] UI 遵循 CLAUDE.md 的 CSS 规范
- [ ] Tauri 编译 release 版本无错
- [ ] 没有浏览器控制台错误

---

## 后续工作（Phase 2+）

- 执行引擎（run_agent_flow）
- 实时执行监视
- 条件分支
- 数据流传递
- 高级功能（循环、暂停点、版本管理）
