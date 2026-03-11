# Claw Browser

> 专为 AI Agent 打造的桌面浏览器，配套 OpenClaw 使用。

---

## 这是什么

Claw Browser 是一款轻量桌面浏览器，核心目的只有一个：**让 AI Agent 能够操控浏览器完成网页任务**。

普通浏览器是给人用的，Claw Browser 是给 AI 用的——同时，人也可以坐在旁边看着，随时接管。

---

## 大虾是谁

**大虾**是内置在 OpenClaw 中的 AI 网页助手角色。当你对 OpenClaw 说：

- 「大虾帮我查一下…」
- 「大妈帮我比比价格…」
- 「大虾帮我看看这个…」

大虾就会接管 Claw Browser，帮你完成搜资讯、比价格、追热点、提取内容、填写表单……一系列网页任务。

你不需要动手，盯着屏幕看就行。

---

## 核心能力

**AI 可以做什么：**

- 打开任意网页、搜索内容
- 点击按钮、填写表单、选择下拉框
- 滚动页面、等待元素加载
- 提取列表数据、读取文章正文
- 执行任意 JavaScript

**人随时可以：**

- 接管浏览器，手动操作
- AI 暂停等待，不干扰
- 操作完成后，让 AI 继续

---

## 一键安装 OpenClaw

Claw Browser 内置了 OpenClaw 安装向导。**首次启动时，如果检测到本地 OpenClaw 尚未运行，应用会自动弹出安装引导页**，无需手动折腾终端。

### 安装流程

点击「开始安装」后，应用会按以下顺序自动检测你的环境并选择最合适的策略：

| 你的环境 | 安装策略 |
|---------|---------|
| 已有 Node.js ≥ 22 | 直接用系统 npm 全局安装，零侵入 |
| 已有 fnm | 用你的 fnm 安装 Node.js 22，`fnm ls` 可见 |
| 已有 nvm | 用你的 nvm 安装 Node.js 22，`nvm ls` 可见 |
| 以上均无 | 用应用内置 fnm（独立隔离目录）安装 Node.js 22 |

> 无论哪种策略，安装完成后应用都会尝试将 `openclaw` 命令自动添加到终端全局 PATH（软链到 `/usr/local/bin`），之后在终端直接输入 `openclaw` 即可使用。

### 完成初始化

npm 安装完成后，应用会提示你在终端执行：

```bash
openclaw onboard
```

这一步是 OpenClaw 的首次配置（包含一个交互式安全确认），完成后 gateway 即可启动。之后回到应用点击「检测连接」即可开始使用。

### 后续重启

OpenClaw gateway 停止后，在终端执行以下命令重新启动：

```bash
openclaw gateway start
```

或在应用「设置」页点击「检测并修复 Gateway 配置」→「重启 Gateway」一键完成。

---

## 使用方式

1. 启动 Claw Browser（首次使用会自动引导安装 OpenClaw）
2. 打开 OpenClaw，连接到本地浏览器
3. 对大虾说出你的任务

浏览器会在后台自动完成操作，你只需要等结果。

---

## 下载

前往 [Releases](../../releases) 下载对应平台的安装包（macOS / Windows）。

---

## macOS 安装后无法打开？

由于应用暂未签名，macOS 会阻止首次启动。按以下步骤解除限制：

**1. 打开终端（Terminal）**

**2. 输入下面的命令，注意末尾有一个空格，先不要按回车：**

```
sudo xattr -rd com.apple.quarantine
```

**3. 将 `claw-browser` 的应用图标直接拖入终端窗口**

终端会自动填入应用路径，最终命令看起来像这样：

```
sudo xattr -rd com.apple.quarantine /Applications/claw-browser.app
```

**4. 按下回车，输入 Mac 开机密码（输入时屏幕不显示字符），再按回车**

**5. 重新打开应用即可**

---

## 本地开发

```bash
pnpm install
pnpm tauri dev
```

构建：

```bash
pnpm tauri build
```

---

## 相关项目

- [OpenClaw](https://github.com/OpenClaw) — AI Agent 平台，大虾的大本营
