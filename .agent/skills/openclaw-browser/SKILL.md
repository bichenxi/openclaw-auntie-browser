---
name: openclaw-browser
description: OpenClaw 专属浏览器项目开发约定与 Tauri/Vue 扩展方式。在为本仓库添加功能、调用 Tauri 命令、改路由或状态时使用。
---

# OpenClaw Auntie Browser 开发技能

## 项目身份

- 仓库：OpenClaw 配套专属浏览器，干净骨架，无历史业务逻辑。
- 栈：Tauri 2 + Vue 3 + TypeScript + Vite + Pinia + UnoCSS + Element Plus。

## 前端约定

- **路由**：`src/pages/` 下 `.vue` 即路由，由 unplugin-vue-router 自动生成，无需手写路由表。
- **布局**：`src/layouts/default.vue` 为默认布局，新页面默认套用；布局切换在 `vite.config.ts` 的 Layouts 插件里改 `defaultLayout`。
- **状态**：Pinia stores 放在 `src/stores/`，可被 unplugin-auto-import 自动导入。
- **样式**：UnoCSS 原子类为主，主题色见 `unocss.config.ts` 的 `theme.colors`（如 `primary`）；全局样式在 `src/styles/main.css`。

## Tauri 扩展

- **命令定义**：在 `src-tauri/src/lib.rs` 中写 `#[tauri::command] fn 命令名(...) -> ...`，并在 `tauri::generate_handler![...]` 中注册。
- **前端调用**：`import { invoke } from '@tauri-apps/api/core'`，然后 `await invoke('命令名', { 参数 })`。
- **依赖**：新 Rust 依赖加到 `src-tauri/Cargo.toml`；插件用 `tauri_plugin_*`，在 `lib.rs` 的 `Builder::default().plugin(...)` 中初始化。

## 新增页面/功能时的检查

- 新页面放在 `src/pages/` 或 `src/pages/子目录/`，保证文件名与期望路径一致。
- 需要全局状态时在 `src/stores/` 新增 store。
- 需要桌面端能力（系统 API、本地服务等）时在 Tauri 侧加 command，再在前端 invoke。

## 添加 Tauri 命令（清单）

1. **Rust 侧**（`src-tauri/src/lib.rs`）  
   - 新增：`#[tauri::command] fn 命令名(参数) -> 返回类型 { ... }`  
   - 在 `tauri::generate_handler![..., 命令名]` 中追加注册  
2. **前端调用**：`const res = await invoke<返回类型>('命令名', { 参数 })`  
3. **异步命令**：Rust 用 `async fn`，前端同样 `invoke` 即可。  
4. **需要 AppHandle**：命令签名为 `fn 命令名(app: AppHandle, ...)`，Tauri 自动注入。
