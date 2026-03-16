---
name: oclaw
description: Oclaw 项目开发约定与 Tauri/Vue 架构规范。在为本仓库添加功能、调用 Tauri 命令、改路由或状态时使用；新增代码必须符合本技能中的前端与 Rust 规范。
---

# Oclaw 开发技能

## 项目身份

- 仓库：Oclaw，与 OpenClaw 结合使用的桌面浏览器壳。
- 栈：Tauri 2 + Vue 3 + TypeScript + Vite + Pinia + UnoCSS + Element Plus。
- 内置 HTTP 服务：`api.rs` 在 `127.0.0.1:18790` 暴露浏览器控制接口，供 OpenClaw Agent 通过 curl 调用（见 `openclaw-skill/SKILL.md`）。

---

## 一、前端架构规范

### 1.1 目录结构（必须遵守）

```
src/
├── api/              # Tauri invoke 封装层，按领域分文件，不在组件里直接 invoke
├── components/       # 小型可复用 UI 组件（TabBar、AIConsole 等）
├── composables/      # 组合式逻辑（useUrlInput 等），以 use 前缀命名
├── layouts/          # 布局组件，default.vue 为默认布局
├── pages/            # URL 路由页（unplugin-vue-router 自动注册），目前只有 index.vue
├── stores/           # Pinia stores
├── utils/            # 纯工具函数（http.ts 等）
├── views/            # 应用级全屏面板（通过 specialView 状态切换，不是 URL 路由）
└── styles/           # 全局样式
```

⚠️ **`views/` vs `pages/` 的区别：**
- `pages/` 下的文件会被 unplugin-vue-router 自动注册为 URL 路由（如 `/`）。
- `views/` 存放通过 Pinia `specialView` 状态切换的全屏面板（OpenclawPage、SettingsPage、SetupPage、SkillsPage），**不注册 URL 路由**，也可被 layout 内嵌复用（如侧边栏中的 OpenclawPage）。
- ❌ 不要把应用面板放进 `pages/`（会产生 URL 路由跳转，不适合桌面 app）。
- ❌ 不要把 Page 级组件放进 `components/`（语义错误，components 只放小型复用组件）。

### 1.2 specialView 视图切换模式

布局通过 Pinia `useTabsStore` 的 `specialView` ref 在多个全屏面板与网页 Tab 之间切换，不使用 vue-router 导航：

```typescript
// stores/tabs.ts
export type SpecialView = 'openclaw' | 'settings' | 'skills' | 'setup'
const specialView = ref<SpecialView | null>(null)

// 切换到某个面板（同时隐藏当前 webview）
await store.switchToSpecialView('openclaw')

// null 且 activeTabId 为 null = 首页
const isHome = computed(() => activeTabId.value === null && specialView.value === null)
```

`default.vue` 中按优先级渲染：

```
setup → openclaw → settings → skills → RouterView（首页） → webview 加载过渡
```

新增全屏面板时：
1. 在 `src/views/` 新建 `XxxPage.vue`
2. 在 `SpecialView` 类型中加入新值
3. 在 `default.vue` 的 `v-if` 链中加入
4. 在 TabBar 中加入触发按钮

### 1.3 当前 API 层文件

`src/api/` 按领域分文件，**禁止在组件/views 中直接 `invoke`**：

| 文件 | 封装内容 |
|------|----------|
| `webview.ts` | createTabWebview / show/hide/closeWebview / resizeAllWebviews / evalInWebview / setActiveTabLabel |
| `openclaw.ts` | checkOpenclawAlive / openclawSendV1 / setAiPaused |
| `installer.ts` | startInstall / cancelInstall / checkOpenclawInstalled |
| `skills.ts` | listSkills / readSkillFile / writeSkillFile / createSkill / deleteSkill / getOpenclawGatewayToken 等 |
| `profile.ts` | getCurrentProfile / setCurrentProfile |
| `sidecar.ts` | sidecar 相关命令 |
| `app.ts` | greet / onWebviewClick 等杂项 |

### 1.4 当前 Pinia Stores

| Store | 用途 |
|-------|------|
| `tabs.ts` | tabs 列表、activeTabId、**specialView**、sidebarOpen、aiPaused，所有导航行为 |
| `settings.ts` | OpenClaw Token / SessionKey / BaseUrl（localStorage 持久化） |
| `profile.ts` | 浏览器身份（default/work/personal） |
| `recording.ts` | 操作录制步骤 |
| `installer.ts` | fnm 安装向导状态（steps、logs、installing、isInstalled） |
| `openclaw.ts` | AI 消息流（messages、sending、sendError） |
| `app.ts` | 应用级全局状态 |

### 1.5 组件约定

- 统一使用 `<script setup lang="ts">`，不使用 Options API。
- `src/components/` 放小型可复用组件（TabBar、AIConsole）。
- `src/views/` 放全屏应用面板（OpenclawPage、SettingsPage、SetupPage、SkillsPage）。
- 纯逻辑抽到 `composables/` 或 `utils/`，不在页面/视图里堆业务逻辑。

### 1.6 新增前端功能检查

- [ ] 新全屏面板放在 `src/views/`，已加入 `SpecialView` 类型与 `default.vue` v-if 链。
- [ ] 新页面（URL 路由）放在 `src/pages/`。
- [ ] 新 Tauri 调用封装到 `src/api/` 对应领域文件，带 TypeScript 类型。
- [ ] 新全局状态在 `src/stores/` 定义。
- [ ] 纯逻辑在 `composables/` 或 `utils/`。

---

## 二、Rust 架构规范

### 2.1 当前模块结构（实际）

```
src-tauri/src/
├── main.rs               # 入口，调用 lib::run()
├── lib.rs                # mod 声明 + manage() + invoke_handler + run()
├── config.rs             # 常量（TAB_BAR_HEIGHT、LEFT_PANEL_WIDTH、CHROME_USER_AGENT 等）
├── api.rs                # HTTP 服务（127.0.0.1:18790），所有浏览器控制端点
├── app.rs                # greet、on_webview_click、emit_stream_item、simulate_stream
├── openclaw.rs           # openclaw_connect/disconnect/send_chat，WebSocket 连接管理
├── openclaw_http.rs      # check_openclaw_alive / openclaw_send_v1（reqwest，no_proxy）
├── openclaw_process.rs   # start/stop/is_openclaw_running，进程生命周期管理
├── installer.rs          # fnm sidecar 安装流程（start_install/cancel_install/check_openclaw_installed）
├── profile.rs            # 浏览器身份（get/set_current_profile）
├── skills.rs             # Skill 文件管理（list/read/write/create/delete/install）
├── bridge.js             # 注入 webview 的 JS（snapshot/extract/highlight 等）
├── stealth.js            # 反指纹注入脚本
└── webview/
    ├── mod.rs
    ├── commands.rs       # create_tab_webview, show/hide/close_webview, resize, eval, snapshot
    └── rect.rs           # calc_webview_rect（计算 webview 位置与大小）
```

### 2.2 lib.rs 职责

`lib.rs` 只做三件事：
1. `mod` 声明所有模块
2. `.manage(状态)` 注册共享状态
3. `generate_handler![所有命令]` 注册 Tauri 命令

新命令实现**禁止**直接写在 `lib.rs`，要放进对应领域模块。

### 2.3 HTTP 服务（api.rs）

`api::spawn_http_server` 在 setup 回调中启动，监听 `127.0.0.1:18790`。
所有浏览器控制端点在此定义（navigate/snapshot/click/type/scroll/select/eval/highlight/wait/extract/extract-text/back/forward）。
修改端点后**必须同步更新 `openclaw-skill/SKILL.md`**（见 `/skill-sync` 命令）。

### 2.4 共享状态（Managed State）

| 类型 | 用途 |
|------|------|
| `ActiveTabLabel` | 当前活动 webview 的 label |
| `PendingSnapshot` | `/snapshot` 一次性 channel sender |
| `PendingEvalResult` | `/eval` 一次性 channel sender |
| `AiPaused` | AI 是否被暂停（AtomicBool） |
| `OpenClawState` | WS 连接与消息 channel |
| `OpenClawProcess` | openclaw 子进程句柄 |
| `InstallerState` | fnm 安装进程状态与取消 channel |

### 2.5 新增 Tauri 命令检查

- [ ] 实现写在对应领域模块，不在 `lib.rs`。
- [ ] 在 `lib.rs` 的 `generate_handler![...]` 中注册。
- [ ] 需要共享状态用 `State<T>` 注入，新状态在 `.manage()` 中添加。
- [ ] 新常量加入 `config.rs`，不硬编码。
- [ ] 前端已通过 `src/api/` 封装并带类型调用。

---

## 三、CSS 与 UI 规范

**必须优先使用 UnoCSS 原子类，禁止在新代码中写 `<style scoped>` 手写 CSS。**

### 3.1 主题色

来自 `unocss.config.ts`：

| Token | 值 | 用途 |
|-------|-----|------|
| `text-secondary` / `bg-secondary` | `#5f47ce` | 主紫色，品牌主色 |
| `text-primary` / `bg-primary` | `#1a00ff` | 主蓝色 |
| `text-accent` / `bg-accent` | `#ef4444` | 红色/危险 |
| `text-neutral-*` / `bg-neutral-*` | 灰色系 | 辅助文字/背景 |

### 3.2 内置 Shortcut

| 类名 | 含义 |
|------|------|
| `btn` | 标准按钮 |
| `btn-plain` | 朴素按钮 |
| `flex-center` | `flex justify-center items-center` |
| `xy-center` | 绝对定位居中 |
| `transition` | 标准过渡动画 |

### 3.3 允许 `<style scoped>` 的情况

1. 复杂 keyframes 动画
2. `:deep()` 覆盖第三方组件内部样式
3. `-webkit-app-region: drag`（Tauri 窗口拖拽，UnoCSS 无法表达）

### 3.4 图标与 Logo

- 项目 Logo：`public/logo.png`（螃蟹形象），用法：
  ```html
  <img src="/logo.png" class="w-9 h-9 rounded-[10px] object-cover shadow" alt="logo" />
  ```
- 图标优先使用内联 SVG（`stroke="currentColor"`），或 `unplugin-icons` 的 `icon-local-*`。

---

## 四、添加新功能总清单

1. **前端视图面板**：新建 `src/views/XxxPage.vue` → 加 `SpecialView` 类型 → 加 `default.vue` v-if → 加 TabBar 按钮。
2. **URL 页面**：新建 `src/pages/xxx.vue`，路由自动生成。
3. **Tauri 命令**：Rust 模块中实现 → `lib.rs` 注册 → `src/api/` 封装 → 组件调用 API 函数。
4. **修改 HTTP 端点**：改 `api.rs` → 同步更新 `openclaw-skill/SKILL.md`（运行 `/skill-sync`）。
5. **新常量**：加入 `config.rs`，不硬编码。
