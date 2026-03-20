# Agent Flow System — 快速入门指南

**最后更新**: 2026-03-20
**目标受众**: 开发团队、架构师、产品经理

---

## 📋 文档导航

本次架构设计包含 **3 个核心文档**，按阅读顺序推荐：

### 1️⃣ **AGENT_FLOW_ARCHITECTURE.md** （架构师/技术lead必读）
- **长度**: ~450 行，15-20 分钟阅读
- **内容**: 完整的系统架构设计
- **关键章节**:
  - 二、数据模型设计 → 理解系统核心数据结构
  - 六、渐进式实施路线图 → Phase 1-5 的分层实施策略
  - 五、Agent 间协作执行引擎 → 理解流程执行机制

### 2️⃣ **AGENT_FLOW_API_SCHEMA.json** （前后端开发者必读）
- **长度**: JSON Schema 格式，易于验证
- **用途**: 作为开发时的类型定义参考
- **包含**:
  - AgentFlow 完整结构定义
  - 所有节点类型的 schema
  - FlowExecution 执行状态模型
  - 示例数据

### 3️⃣ **AGENT_FLOW_PHASE1_CHECKLIST.md** （项目经理/开发lead必读）
- **长度**: ~300 行，10 分钟阅读
- **用途**: Phase 1 MVP 的详细实施计划
- **包含**:
  - 12 项具体任务分解
  - 前后端工作量估算（8天 + 3.5天）
  - 里程碑和验收标准
  - 每项任务的预期输出

---

## 🎯 5 分钟速览

### 核心概念

**AgentFlow** = DAG（有向无环图）
```
┌─────────────────────┐
│   工作流 Flow       │
├─────────────────────┤
│ • nodes: []  (节点) │
│ • edges: []  (连接) │
│ • metadata   (元数据)│
└─────────────────────┘
```

**节点类型**（3种）
- **AgentNode**: 调用一个已配置的 Agent
- **TaskNode**: 本地操作（数据转换、合并等）
- **GatewayNode**: 条件分支、循环网关

### 核心流程

```
创建流程
  ↓
添加节点 (Agent/Task/Gateway)
  ↓
连接节点（拖拽或手工）
  ↓
编辑节点属性（右侧面板）
  ↓
保存流程 → 磁盘
  ↓
执行流程 → Phase 2
  ↓
监视进度 + 日志 → Phase 2
```

### 技术栈快速参考

| 层级 | 技术 | 新增依赖 |
|------|------|--------|
| UI编辑 | Vue 3 + VueFlow | @vue-flow/core |
| 状态管理 | Pinia | (已有) |
| 样式 | UnoCSS | (已有) |
| 后端API | Tauri commands | (已有) |
| 存储 | 文件系统 | (已有) |

### 实施时间线

```
Week 1-2: Phase 1 MVP (可视化编辑)
  ├─ Day 1: 类型定义 + Store
  ├─ Day 2-3: VueFlow 集成 + 节点组件
  ├─ Day 4: 右侧面板 + 工具栏
  └─ Day 5: Rust 后端 + 文件存储

Week 3: Phase 2 (单Agent执行)
  ├─ 执行引擎基础版
  ├─ 状态监视 UI
  └─ 日志展示

Week 4-5: Phase 3 (多Agent协作)
  ├─ DAG 拓扑排序
  ├─ 数据传递机制
  └─ 完整执行引擎

Week 6+: Phase 4+ (条件分支、高级功能)
```

---

## 🚀 如何开始

### 对于产品经理

1. 阅读 `AGENT_FLOW_ARCHITECTURE.md` 的 **一、概述** 部分
2. 理解 **六、渐进式实施路线图** 中的 5 个阶段
3. 根据需求优先级调整实施时间线

**关键问题**:
- MVP（Phase 1）最快能交付什么？→ 可视化编辑 + 保存加载
- 何时支持实际执行？→ Phase 2（第 3 周）
- 何时支持多 Agent 协作？→ Phase 3（第 4-5 周）

### 对于架构师

1. 逐行阅读 `AGENT_FLOW_ARCHITECTURE.md`
2. 特别关注:
   - **二、数据模型** → 是否符合现有系统设计
   - **三、前端架构** → 组件结构是否合理
   - **四、后端架构** → Tauri commands 是否足够
   - **五、执行引擎** → DAG 执行策略是否可行
3. 如有建议，在 Phase 1 开始前提出

**关键决策点**:
- [ ] VueFlow 作为编辑器是否采纳？
- [ ] 数据存储在 `~/.openclaw/flows/` 是否合理？
- [ ] 阶段式实施是否可接受？

### 对于前端开发者

1. 先阅读 `AGENT_FLOW_ARCHITECTURE.md` **三、前端架构** 部分
2. 理解 Pinia Store 的数据流
3. 查看 `AGENT_FLOW_PHASE1_CHECKLIST.md` 中的任务列表
4. 按顺序完成:
   - 数据模型 → Pinia Store → VueFlow → 组件 → 集成

**必读部分**:
- 第 2.1-2.3 节：数据结构定义
- 第 3.1-3.3 节：前端组件和 Store 设计
- Checklist 中的 "前端任务" 部分

**起始代码**:
```bash
# 从这里开始
src/types/agentFlow.ts        # 第一步：类型定义
src/stores/agentFlows.ts      # 第二步：Pinia Store
src/composables/useFlowEditor.ts  # 第三步：VueFlow 集成
```

### 对于后端开发者

1. 重点阅读 `AGENT_FLOW_ARCHITECTURE.md` **四、后端架构** 部分
2. 查看 `AGENT_FLOW_API_SCHEMA.json` 了解数据格式
3. 从 Checklist 中的"后端任务"开始

**必读部分**:
- 第 4.1-4.3 节：Rust 模块实现
- JSON Schema 的完整定义

**起始代码**:
```bash
# 从这里开始
src-tauri/src/agent_flows.rs  # 新建模块
# 实现 8 个 Tauri commands
```

---

## 📊 Phase 1 MVP 工作量

| 组件 | 前端工程师 | 后端工程师 |
|------|----------|---------|
| 数据模型 | 0.5天 | 0.5天 |
| Pinia Store | 1天 | - |
| VueFlow 集成 | 1.5天 | - |
| 节点组件 | 1.5天 | - |
| 面板 + 工具栏 | 2天 | - |
| **前端小计** | **6.5天** | - |
| Tauri Commands | - | 1.5天 |
| 文件存储 | - | 1.5天 |
| 文件 API | 0.5天 | 0.5天 |
| **后端小计** | - | **3.5天** |
| **总计** | **7天** | **3.5天** |

---

## ⚠️ 常见问题

### Q: 为什么分 5 个阶段？不能一次性实现吗？

**A**: 分阶段有几个好处：
- 快速交付 MVP（2 周可验证可行性）
- 降低技术风险（逐步复杂度提升）
- 获得用户反馈后调整
- 团队可以边学边做

### Q: Phase 1 能做什么？

**A**: Phase 1（MVP）可以：
- ✅ 创建流程
- ✅ 可视化编辑节点和边
- ✅ 编辑节点属性
- ✅ 保存到磁盘
- ✅ 加载已保存的流程
- ❌ 执行流程（需要 Phase 2）
- ❌ Agent 间通信（需要 Phase 3）

### Q: VueFlow 会不会不够灵活？

**A**: VueFlow 提供了高度的可定制性：
- 自定义节点类型（我们定义了 3 种）
- 自定义边渲染
- 事件系统完整
- 如果确实有限制，可在 Phase 2 切换到其他库

### Q: 执行引擎什么时候实现？

**A**: 按计划：
- Phase 1：无执行（仅编辑）
- Phase 2（Week 3）：单 Agent 执行
- Phase 3（Week 4-5）：多 Agent 协作执行

### Q: 数据从哪里来？

**A**: 初期的数据流向：
- 用户输入 → Pinia Store → VueFlow 画布
- 画布编辑 → Pinia Store → 保存到 JSON 文件
- 打开文件 → JSON 反序列化 → Pinia Store → 画布展示

### Q: 如何与现有 Agent 系统集成？

**A**:
- AgentNode 通过 `agentWork` 字段引用已有的 Agent
- 执行时调用 OpenClaw CLI：`openclaw chat <work> "prompt"`
- 获取返回结果后传给下一个 Agent

---

## 🔄 下一步

### 立即可做（今天）
- [ ] 架构师审查设计文档
- [ ] PM 确认阶段式实施计划
- [ ] Dev lead 分配人员

### 本周开始（Week 1）
- [ ] 前端开发者开始 Phase 1
- [ ] 后端开发者开始 Phase 1
- [ ] 每天 standup 同步进度

### Phase 1 交付（Week 2 末）
- [ ] 可创建和编辑工作流
- [ ] 可保存加载流程
- [ ] 提交代码审查
- [ ] 收集反馈

### Phase 2 规划（Week 3）
- [ ] 基于 Phase 1 反馈调整
- [ ] 开始执行引擎设计
- [ ] 实现运行/监视功能

---

## 📞 技术支持

如有问题或建议：

1. **数据模型问题**: 参考 `AGENT_FLOW_API_SCHEMA.json`
2. **实施细节**: 查看 `AGENT_FLOW_PHASE1_CHECKLIST.md` 的对应部分
3. **架构疑问**: 回到 `AGENT_FLOW_ARCHITECTURE.md` 对应章节
4. **代码示例**: 文档中有完整的 TypeScript/Rust 代码片段

---

## 📚 相关文档清单

- ✅ `AGENT_FLOW_ARCHITECTURE.md` — 完整架构设计
- ✅ `AGENT_FLOW_API_SCHEMA.json` — API 类型定义
- ✅ `AGENT_FLOW_PHASE1_CHECKLIST.md` — 实施清单
- ✅ `AGENT_FLOW_QUICKSTART.md` — 本文件

---

**祝开发顺利！** 🚀

---

*最后更新: 2026-03-20*
*维护者: Architecture Team*
