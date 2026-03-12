# Oclaw — Claude Code 项目规范

## 技术栈

- **Tauri 2** + **Vue 3** + **TypeScript**
- **UnoCSS**（原子化 CSS，配置见 `unocss.config.ts`）
- **Pinia**（状态管理）
- **unplugin-auto-import**（Vue/VueRouter/VueUse API 无需手动 import）
- **unplugin-vue-router**（文件路由，`src/pages/`）
- **vite-plugin-vue-layouts**（布局，`src/layouts/`）
- **Element Plus**（组件库，按需自动导入）

## CSS 规范 ⚠️ 重要

**必须优先使用 UnoCSS 原子类，禁止在新代码中写 `<style scoped>` 手写 CSS。**

```vue
<!-- ✅ 正确 -->
<div class="flex items-center gap-2 px-4 py-2 rounded-lg bg-secondary text-white">

<!-- ❌ 错误 -->
<div class="my-box">
<style scoped>
.my-box { display: flex; align-items: center; ... }
</style>
```

### 常用 UnoCSS 配置（来自 unocss.config.ts）

**主题色（直接用类名）：**
- `text-secondary` / `bg-secondary` → 主紫色 `#5f47ce`
- `text-primary` / `bg-primary` → 主蓝色 `#1a00ff`
- `text-accent` / `bg-accent` → 红色 `#ef4444`
- `text-neutral-*` / `bg-neutral-*` → 灰色系

**自定义快捷方式：**
- `btn` → 标准按钮样式
- `btn-plain` → 朴素按钮样式
- `flex-center` → `display:flex; justify-content:center; align-items:center`
- `xy-center` → 绝对定位居中
- `transition` → 标准过渡动画

**动画/过渡：** 使用 `transition` 快捷类或 `transition-all duration-150`

### 只在以下情况允许 `<style scoped>`
1. 复杂动画 keyframes（UnoCSS 无法表达）
2. 深度选择器 `:deep()`
3. 第三方组件样式覆盖

## 文件结构

```
src/
├── pages/          # 路由页面（unplugin-vue-router 自动注册）
├── layouts/        # 布局组件
├── components/     # 通用组件
├── stores/         # Pinia stores
├── api/            # Tauri invoke 封装
├── composables/    # 组合式函数
└── styles/         # 全局样式

src-tauri/
├── src/            # Rust 源码
└── icons/          # 应用图标
```

## 状态管理约定

- `src/stores/tabs.ts` — Tab 和 `specialView`（'openclaw'|'settings'|null）
- `src/stores/settings.ts` — OpenClaw Token/SessionKey/BaseUrl（localStorage 持久化）
- `src/stores/profile.ts` — 浏览器身份（default/work/personal）
- `src/stores/recording.ts` — 操作录制步骤

## OpenClaw Agent Skill 文件

### 文件用途

`openclaw-skill/` 目录下的 `.md` 文件是 **OpenClaw AI Agent 的提示词（Skill）**，
不是给 Claude Code 的指令。OpenClaw 在接到用户任务时会读取这些文件，
了解如何通过 `curl http://127.0.0.1:18790` 控制 Oclaw。

```
openclaw-skill/
└── SKILL.md      ← 通用浏览器操控技能（所有站点通用 API）
```

### Claude Code 的职责 ⚠️

**当你修改了 `src-tauri/src/api.rs` 中的 HTTP 端点（新增/修改/删除），
必须同步更新对应的 skill 文件，否则 OpenClaw Agent 会调用不存在的接口。**

| 代码改动 | 需要更新的 skill 文件 |
|---------|----------------------|
| 修改通用端点（`/snapshot`, `/click` 等） | `openclaw-skill/SKILL.md` 的"API 快速参考"表格 |
| 新增站点专属端点 | 新建对应 skill 文件，并在 `SKILL.md` 站点表格中登记 |

### 可用的 Skill 命令

项目内置了以下 Claude Code 斜杠命令（`.claude/commands/`）：

- `/skill-read` — 读取所有 skill 文件并输出现状摘要 + 代码一致性检查
- `/skill-sync` — 对比 `api.rs` 与 skill 文件，找出差异并自动同步更新
- `/skill-add <站点名>` — 为新站点创建专属 skill 文件

**在完成任何涉及 `api.rs` 的修改后，建议运行 `/skill-sync` 确保同步。**

## 图标

项目 Logo 使用 `public/logo.png`（Oclaw 猫爪形象），在组件中用：
```html
<img src="/logo.png" class="w-9 h-9 rounded-[10px] object-cover shadow" alt="logo" />
```

## Tauri 配置注意

- `LEFT_PANEL_WIDTH = 0.0`（无侧边栏，webview 全宽）
- `TAB_BAR_HEIGHT = 88.0`（profile 行 + TabBar 合计高度）
- webview 显示/隐藏必须通过 `webviewApi.showWebview/hideWebview` 控制
