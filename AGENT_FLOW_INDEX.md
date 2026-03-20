# Agent Flow 编排系统 — 设计文档索引

**项目**: Oclaw（Tauri + Vue 3 浏览器）
**任务**: Task #4 - 技术架构设计
**完成日期**: 2026-03-20
**状态**: ✅ 完成，可进入 Phase 1 开发

---

## 📦 交付成果概览

本次架构设计包含 **4 个核心文档**，总计 **60KB** 内容，涵盖从概念到代码的完整设计体系。

### 文档清单与大小

| 文档 | 大小 | 目标读者 | 预计阅读 | 用途 |
|------|------|---------|--------|------|
| **AGENT_FLOW_ARCHITECTURE.md** | 34KB | 架构师、Tech Lead | 20-30分 | ⭐ 主设计文档 |
| **AGENT_FLOW_API_SCHEMA.json** | 10KB | 前后端开发者 | 10分 | API 类型定义 |
| **AGENT_FLOW_PHASE1_CHECKLIST.md** | 8.7KB | PM、项目经理 | 10分 | 实施计划 |
| **AGENT_FLOW_QUICKSTART.md** | 8.0KB | 全体团队 | 10分 | 快速导航 |

---

## 🎯 文档内容速查

### 1. AGENT_FLOW_ARCHITECTURE.md（主文档）

**内容结构**：
```
一、概述 ........................ 系统目标和核心原则
二、数据模型设计 ................ AgentFlow/Node/Edge 的完整定义
  2.1 核心数据结构 ............. TypeScript 接口定义
  2.2 存储结构 ................. 文件系统组织和 JSON Schema
三、前端架构设计 ................ Vue/VueFlow 集成方案
  3.1 新增组件列表 ............. 12 个新组件的职责
  3.2 Pinia Store 扩展 ......... agentFlows store 的完整实现
  3.3 VueFlow 集成配置 ......... 节点类型、初始化、同步
  3.4 路由注册 ................. 页面和导航集成
四、后端架构设计 ................ Rust/Tauri 实现
  4.1 新增 Module .............. agent_flows.rs 模块
  4.2 Agent Flows 模块实现 ...... 数据结构和 8 个 Tauri commands
  4.3 Cargo.toml 依赖更新 ...... 所需依赖列表
五、Agent 间协作执行引擎设计 ..... DAG 执行策略
  5.1 执行流程设计 ............. 7 步执行过程图
  5.2 Pseudo Code .............. 执行引擎伪代码
  5.3 数据传递机制 ............. 节点间数据流通
六、渐进式实施路线图 ............ 5 个阶段的分层计划
  Phase 1 ...................... MVP（2周）
  Phase 2 ...................... 单Agent执行（1周）
  Phase 3 ...................... 多Agent协作（2周）
  Phase 4 ...................... 条件分支（1周）
  Phase 5 ...................... 高级功能（可选）
七、风险与缓解 ................. 表格式风险评估
八、API 快速参考 ............... Tauri Commands 汇总
九、关键决策文档 ............... VueFlow 选择理由等
十、总结 ....................... 架构亮点和后续行动
```

**重点章节**：
- 🔴 **必读**: 一、二、六（系统基础 + 实施计划）
- 🟡 **前端必读**: 三（UI 架构）
- 🟡 **后端必读**: 四（Rust 实现）
- 🟢 **参考**: 五、七、八（深入理解）

---

### 2. AGENT_FLOW_API_SCHEMA.json（API 定义）

**包含内容**：
```json
{
  "definitions": {
    "AgentFlow": { ... },           // 完整工作流定义
    "AgentNode": { ... },           // Agent 节点类型
    "TaskNode": { ... },            // 任务节点类型
    "GatewayNode": { ... },         // 网关节点类型
    "FlowEdge": { ... },            // 边定义
    "Position": { ... },            // 坐标定义
    "FlowExecution": { ... },       // 执行实例定义
    "NodeResult": { ... },          // 节点执行结果
    "ExecutionLog": { ... }         // 日志条目
  },
  "examples": {
    "AgentFlow": { ... }            // 完整示例
  }
}
```

**用途**：
- ✅ 前端：类型校验、IDE 自动完成
- ✅ 后端：数据验证、序列化
- ✅ 文档：API 契约定义

**如何使用**：
```bash
# 在 IDE 中引用 (VS Code)
// 在 tsconfig.json 中
{
  "compilerOptions": {
    "types": ["./AGENT_FLOW_API_SCHEMA.json"]
  }
}

# 或作为 Rust 的参考
// 检查 src-tauri/src/agent_flows.rs 中的结构定义
```

---

### 3. AGENT_FLOW_PHASE1_CHECKLIST.md（实施计划）

**包含内容**：
```
任务分解（12项）
├─ 前端（7项）
│  ├─ 数据模型与类型定义 (1天)
│  ├─ Pinia Store 实现 (1天)
│  ├─ VueFlow 集成 (1.5天)
│  ├─ 节点自定义渲染 (1.5天)
│  ├─ 右侧属性面板 (1天)
│  ├─ 工具栏 (1天)
│  └─ 页面集成 (1天)
└─ 后端（5项）
   ├─ Agent Flows 模块结构 (1天)
   ├─ Tauri Commands (1天)
   ├─ 文件存储 (1天)
   └─ 前端 API 层 (0.5天)

集成测试流程
├─ 创建流程
├─ 添加节点
├─ 编辑属性
├─ 保存加载
└─ 验收标准

里程碑规划
└─ 总计 10 个工作日（前端8天 + 后端3.5天）
```

**工作量汇总**：
- 前端：8 个工作日
- 后端：3.5 个工作日
- 总计：11.5 个工作日 ≈ 2 周（含同步、审查、测试）

**每项任务包含**：
- ✅ 详细描述
- ✅ 验收标准
- ✅ 依赖关系
- ✅ 预计工时

---

### 4. AGENT_FLOW_QUICKSTART.md（导航指南）

**包含内容**：
```
文档导航 ...................... 3 个核心文档的阅读指南
5 分钟速览 .................... 核心概念速讲
如何开始（分角色）
├─ 产品经理 ................. 重点阅读 Architecture §6
├─ 架构师 ................... 完整阅读整个 Architecture
├─ 前端开发者 ............... 重点阅读 Architecture §3
└─ 后端开发者 ............... 重点阅读 Architecture §4
工作量总结 ................... Phase 1 估算表
常见问题 (FAQ) ............... 5 个高频问题答案
下一步行动清单 ............... 今天/本周/Phase 结束
技术支持 ..................... 问题解答方法
```

**快速导航**：
- 🟢 5分钟了解全貌 → 读"5分钟速览"
- 🟡 10分钟判断可行性 → 读"核心概念" + "实施时间线"
- 🔴 决定启动项目 → 完整阅读 ARCHITECTURE.md

---

## 🔍 按角色快速导读

### 👔 产品经理

**必读顺序**：
1. AGENT_FLOW_QUICKSTART.md（5分钟）
   - 了解这是什么产品功能
   - 理解 5 个实施阶段
2. AGENT_FLOW_ARCHITECTURE.md § 六（10分钟）
   - Phase 1-5 的功能清单
   - 每个阶段的交付物
3. AGENT_FLOW_PHASE1_CHECKLIST.md（10分钟）
   - Phase 1 的工作量和时间线
   - 验收标准

**关键信息**：
- ✅ MVP 可在 2 周交付
- ✅ 支持可视化编辑和保存加载
- ❌ 执行功能需要 Phase 2（第 3 周）
- 💰 总工作量：前端 8 天 + 后端 3.5 天

### 🏗️ 架构师

**必读顺序**：
1. AGENT_FLOW_QUICKSTART.md（5分钟）
   - 技术栈快速参考
2. AGENT_FLOW_ARCHITECTURE.md（25分钟）
   - 从头到尾完整阅读
   - 特别关注数据模型、前后端架构、执行引擎
3. AGENT_FLOW_API_SCHEMA.json（10分钟）
   - 验证数据结构完整性

**关键决策**：
- [ ] 同意 VueFlow 作为编辑器？
- [ ] 同意分 5 个阶段实施？
- [ ] 同意数据存储在 ~/.openclaw/flows/？
- [ ] 执行引擎的 DAG 策略是否可行？

### 💻 前端开发者

**必读顺序**：
1. AGENT_FLOW_QUICKSTART.md（快速定位）
2. AGENT_FLOW_ARCHITECTURE.md § 三（前端架构）
   - 3.1 新增组件
   - 3.2 Pinia Store
   - 3.3 VueFlow 集成
3. AGENT_FLOW_PHASE1_CHECKLIST.md（任务列表）
   - "前端任务"部分
4. AGENT_FLOW_API_SCHEMA.json（数据结构）

**起始工作**：
```typescript
// 优先级顺序
1. src/types/agentFlow.ts              // 类型定义
2. src/stores/agentFlows.ts            // Pinia store
3. src/components/AgentFlowCanvas.vue  // VueFlow 画布
4. src/components/nodes/*              // 自定义节点
5. src/components/FlowProperties.vue   // 属性面板
6. src/views/AgentFlowPage.vue         // 页面集成
```

### 🔧 后端开发者

**必读顺序**：
1. AGENT_FLOW_QUICKSTART.md（快速定位）
2. AGENT_FLOW_ARCHITECTURE.md § 四（后端架构）
   - 4.1 新增 Module
   - 4.2 实现细节
3. AGENT_FLOW_API_SCHEMA.json（数据契约）
4. AGENT_FLOW_PHASE1_CHECKLIST.md（后端任务）

**起始工作**：
```rust
// 优先级顺序
1. src-tauri/src/agent_flows.rs        // 数据结构 + 8 个 commands
2. Cargo.toml 依赖更新
3. src-tauri/src/lib.rs                // 注册 commands
4. src/api/agentFlows.ts               // 前端 API 层
```

---

## 🎬 Phase 1 实施流程

### Week 1（Day 1-5）

```
Day 1
├─ 前端: 类型定义 + Store 框架
└─ 后端: 模块框架 + 数据结构

Day 2-3
├─ 前端: VueFlow 集成 + 节点组件
└─ 后端: Tauri Commands 骨架

Day 4-5
├─ 前端: 面板 + 工具栏 + 集成测试
└─ 后端: 文件存储 + API 完善

Week 2（Day 6-10）

Day 6-7
├─ 前端: 联调测试 + Bug 修复
└─ 后端: 联调测试 + Bug 修复

Day 8-9
├─ 代码审查
├─ 性能优化
└─ 文档完善

Day 10
├─ Final 集成测试
├─ Build & Release
└─ 交付演示
```

### 关键里程碑

| 时间 | 里程碑 | 审查内容 |
|------|--------|--------|
| Day 2 | 数据模型完成 | TS 编译 + TS 检查无错 |
| Day 4 | UI 框架完成 | 页面加载 + 无黑屏 |
| Day 7 | 前后端联调 | 增删改流程可用 |
| Day 9 | 代码审查 | 代码风格 + 注释完整 |
| Day 10 | 交付验收 | 所有验收标准达成 |

---

## ✅ Phase 1 验收清单

### 功能验收
- [ ] 创建新工作流
- [ ] 添加 Agent/Task/Gateway 节点
- [ ] 拖拽移动节点
- [ ] 连接节点（边）
- [ ] 编辑节点属性（名称、Agent选择、参数）
- [ ] 删除节点和边
- [ ] 保存流程到磁盘
- [ ] 加载已保存的流程
- [ ] 删除流程

### 非功能验收
- [ ] TypeScript 无编译错误
- [ ] Tauri 编译成功（dev 和 release）
- [ ] 性能：编辑大流程（50+ 节点）不卡顿
- [ ] 内存：单流程不超过 10MB
- [ ] 响应时间：保存/加载 < 500ms

### UI/UX 验收
- [ ] 页面布局清晰（工具栏 + 画布 + 属性面板）
- [ ] 节点可视化区分（不同图标、颜色）
- [ ] 用户提示清晰（操作成功/失败提示）
- [ ] 响应式设计（适配不同屏幕）
- [ ] 无运行时警告（控制台干净）

### 代码质量验收
- [ ] 代码注释完整
- [ ] Store actions 有 JSDoc
- [ ] 错误处理完善
- [ ] 无硬编码路径
- [ ] 文件名和目录结构规范

---

## 📞 常见问题快速查询

| 问题 | 答案位置 |
|------|---------|
| 什么是 AgentFlow？ | ARCHITECTURE.md § 2.1 |
| 如何开始前端开发？ | QUICKSTART.md "对于前端开发者" |
| Phase 1 需要多久？ | CHECKLIST.md "里程碑"或 QUICKSTART.md "Phase 1 工作量" |
| 为什么用 VueFlow？ | ARCHITECTURE.md § 9.1 |
| 执行引擎如何工作？ | ARCHITECTURE.md § 5 |
| 数据如何传递？ | ARCHITECTURE.md § 5.3 |

---

## 🔗 文件位置（相对项目根目录）

```
.
├── AGENT_FLOW_ARCHITECTURE.md        ← 主设计文档 (34KB)
├── AGENT_FLOW_API_SCHEMA.json        ← API 类型定义 (10KB)
├── AGENT_FLOW_PHASE1_CHECKLIST.md    ← 实施计划 (8.7KB)
├── AGENT_FLOW_QUICKSTART.md          ← 快速导航 (8KB)
│
├── src/types/
│   └── agentFlow.ts                  ← （待创建）TypeScript 类型
├── src/stores/
│   └── agentFlows.ts                 ← （待创建）Pinia Store
├── src/components/
│   ├── AgentFlowCanvas.vue           ← （待创建）VueFlow 画布
│   ├── FlowProperties.vue            ← （待创建）属性面板
│   ├── FlowToolbar.vue               ← （待创建）工具栏
│   └── nodes/
│       ├── AgentFlowNode.vue         ← （待创建）
│       ├── TaskFlowNode.vue          ← （待创建）
│       └── GatewayFlowNode.vue       ← （待创建）
├── src/views/
│   └── AgentFlowPage.vue             ← （待创建）页面
│
└── src-tauri/src/
    └── agent_flows.rs                ← （待创建）Rust 模块
```

---

## 📈 项目进度追踪

使用此矩阵追踪 Phase 1 进度：

```markdown
## Week 1 进度

| 任务 | 负责人 | Day1 | Day2 | Day3 | Day4 | Day5 | 状态 |
|------|--------|------|------|------|------|------|------|
| 类型定义 | @frontend | ✅ | | | | | Done |
| Pinia Store | @frontend | | ✅ | | | | Done |
| VueFlow 集成 | @frontend | | | ✅ | ✅ | | In Progress |
| 节点组件 | @frontend | | | ✅ | ✅ | ✅ | In Progress |
| Rust 模块 | @backend | ✅ | ✅ | | | | Done |
| Tauri Commands | @backend | | ✅ | ✅ | | | In Progress |
| 文件存储 | @backend | | | | ✅ | ✅ | In Progress |
```

---

## 🎉 总结

这份完整的架构设计为 Agent Flow 编排系统奠定了坚实基础：

✅ **清晰的概念**: AgentFlow = DAG，3 种节点类型，5 个实施阶段
✅ **详细的设计**: 前后端完整实现方案，包括代码示例
✅ **可执行的计划**: Phase 1 详细分解到日级别任务
✅ **可验证的标准**: 明确的验收标准和测试清单

**下一步**: 选择你的角色，按照对应的"必读顺序"开始！

---

*文档生成日期: 2026-03-20*
*Version: 1.0*
*Status: Ready for Phase 1 Development*
