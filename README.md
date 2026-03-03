# OpenClaw Auntie Browser

为 **OpenClaw** 配套的专属桌面浏览器，基于 Tauri 2 + Vue 3 的干净骨架，便于扩展与 OpenClaw 相关的浏览器能力。

## 技术栈

| 层级     | 技术 |
|----------|------|
| 前端     | Vue 3 (Composition API) + TypeScript + Vite |
| 路由     | vue-router + unplugin-vue-router（文件路由） |
| 布局     | vite-plugin-vue-layouts |
| 状态     | Pinia |
| UI/样式  | Element Plus + UnoCSS |
| 桌面壳   | Tauri 2 (Rust) |

## 环境要求

- Node.js >= 18
- pnpm（推荐）或 npm/yarn
- Rust 稳定版
- 系统依赖见 [Tauri 文档](https://tauri.app/)

## 快速开始

```bash
pnpm install
pnpm tauri dev
```

构建产物：

```bash
pnpm tauri build
```

输出目录：`src-tauri/target/release/bundle/`。

## 项目结构

```
├── src/                 # 前端
│   ├── pages/           # 页面（文件即路由）
│   ├── layouts/         # 布局
│   ├── components/      # 公共组件
│   ├── stores/          # Pinia stores
│   └── styles/          # 全局样式
├── src-tauri/           # Tauri 后端
│   └── src/lib.rs       # 命令入口
├── .agent/              # 项目 Agent 与技能
│   ├── AGENTS.md        # 项目说明与约定
│   └── skills/          # 项目专属技能
└── ...
```

## 扩展说明

- **新页面**：在 `src/pages/` 下新增 `.vue`，路由自动生成。
- **Tauri 命令**：在 `src-tauri/src/lib.rs` 添加 `#[tauri::command]` 并注册到 `generate_handler![]`；前端通过 `invoke('命令名', { ... })` 调用。
- **AI 协作**：见 `.agent/AGENTS.md` 与 `.agent/skills/` 下的技能说明。

## 相关链接

- [Tauri](https://tauri.app/)
- [Vue 3](https://vuejs.org/)
- [UnoCSS](https://unocss.dev/)
- [Element Plus](https://element-plus.org/)
- [Pinia](https://pinia.vuejs.org/)
