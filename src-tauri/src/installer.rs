//! OpenClaw 安装流程
//!
//! 安装策略：
//! 1. 通过登录 shell 检测系统是否已有 Node.js >= 22
//!    ├ 有 → 直接用系统 npm install -g openclaw（无需 fnm）
//!    └ 无 → 用 fnm sidecar 安装 Node 22，再 npm install -g openclaw；
//!            安装完成后尝试将 openclaw 软链到 /usr/local/bin 或 ~/.local/bin
//! 2. openclaw onboard 为交互式 TUI，无法在 app 内自动化。
//!    npm 安装完成后发送 installer:need-onboard 事件，由前端引导用户手动在终端执行。

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::ShellExt;

// ─── 状态 ──────────────────────────────────────────────────────────────────

pub struct InstallerState {
    running: Arc<Mutex<bool>>,
    cancel_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl Default for InstallerState {
    fn default() -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            cancel_tx: Arc::new(Mutex::new(None)),
        }
    }
}

// ─── 事件 Payload ──────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
struct StepPayload {
    step: String,
    status: String,
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
    line: String,
}

#[derive(Clone, serde::Serialize)]
struct ErrorPayload {
    step: String,
    message: String,
}

// ─── emit helpers ──────────────────────────────────────────────────────────

fn emit_step(app: &AppHandle, step: &str, status: &str) {
    let _ = app.emit(
        "installer:step",
        StepPayload { step: step.to_string(), status: status.to_string() },
    );
}

fn emit_log(app: &AppHandle, line: &str) {
    let _ = app.emit("installer:log", LogPayload { line: line.to_string() });
}

fn emit_error(app: &AppHandle, step: &str, message: &str) {
    let _ = app.emit(
        "installer:error",
        ErrorPayload { step: step.to_string(), message: message.to_string() },
    );
}

// ─── 系统检测 ──────────────────────────────────────────────────────────────

/// 返回最合适的登录 shell 路径（$SHELL → zsh → bash → sh）
fn detect_login_shell() -> String {
    if let Ok(s) = std::env::var("SHELL") {
        if std::path::Path::new(&s).exists() {
            return s;
        }
    }
    for sh in &["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if std::path::Path::new(sh).exists() {
            return sh.to_string();
        }
    }
    "/bin/sh".to_string()
}

/// 通过登录 shell 检测 node major 版本。返回 None 表示未安装或无法检测。
fn detect_system_node_major() -> Option<u32> {
    let shell = detect_login_shell();
    let output = std::process::Command::new(&shell)
        .args(["-l", "-c", "node --version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    let trimmed = s.trim().trim_start_matches('v');
    let major_str = trimmed.split('.').next()?;
    major_str.parse::<u32>().ok()
}

// ─── 运行命令并流式输出 ────────────────────────────────────────────────────

/// 用登录 shell 运行单条命令，实时推送 stdout/stderr 到前端。
/// Unix 用 `$SHELL -l -c`，Windows 用 `cmd /C`。
async fn run_login_shell_step(
    app: &AppHandle,
    cmd_str: &str,
    cancel_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    #[cfg(not(target_os = "windows"))]
    let mut child = {
        let shell = detect_login_shell();
        Command::new(&shell)
            .args(["-l", "-c", cmd_str])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("启动命令失败：{}", e))?
    };

    #[cfg(target_os = "windows")]
    let mut child = Command::new("cmd")
        .args(["/C", cmd_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动命令失败：{}", e))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let mut out_lines = BufReader::new(stdout).lines();
    let mut err_lines = BufReader::new(stderr).lines();
    let mut out_done = false;
    let mut err_done = false;

    loop {
        if out_done && err_done {
            break;
        }
        tokio::select! {
            _ = &mut *cancel_rx => {
                let _ = child.kill().await;
                return Err("已取消".to_string());
            }
            line = out_lines.next_line(), if !out_done => {
                match line {
                    Ok(Some(l)) => emit_log(app, &l),
                    _ => out_done = true,
                }
            }
            line = err_lines.next_line(), if !err_done => {
                match line {
                    Ok(Some(l)) => emit_log(app, &l),
                    _ => err_done = true,
                }
            }
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("进程退出码 {}", status.code().unwrap_or(-1)))
    }
}

/// 用 fnm sidecar 运行命令，实时推送 stdout/stderr 到前端。
async fn run_step(
    app: &AppHandle,
    fnm_dir: &str,
    args: &[&str],
    cancel_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    use tauri_plugin_shell::process::CommandEvent;

    let shell = app.shell();
    let mut cmd = shell.sidecar("fnm").map_err(|e| e.to_string())?;
    cmd = cmd.args(["--fnm-dir", fnm_dir]);
    cmd = cmd.args(args);

    let (mut rx, child) = cmd.spawn().map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            _ = &mut *cancel_rx => {
                let _ = child.kill();
                return Err("已取消".to_string());
            }
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(CommandEvent::Stdout(bytes)) => {
                        emit_log(app, String::from_utf8_lossy(&bytes).trim_end());
                    }
                    Some(CommandEvent::Stderr(bytes)) => {
                        emit_log(app, String::from_utf8_lossy(&bytes).trim_end());
                    }
                    Some(CommandEvent::Terminated(payload)) => {
                        let code = payload.code.unwrap_or(-1);
                        return if code == 0 { Ok(()) } else { Err(format!("进程退出码 {}", code)) };
                    }
                    Some(_) => {}
                    None => return Ok(()),
                }
            }
        }
    }
}

// ─── fnm 路径辅助 ──────────────────────────────────────────────────────────

/// 在 fnm 目录下查找 Node 22 安装目录内的 openclaw 二进制。
#[cfg(not(target_os = "windows"))]
fn find_fnm_openclaw_binary(fnm_dir: &str) -> Option<std::path::PathBuf> {
    let node_versions = std::path::Path::new(fnm_dir).join("node-versions");
    let v22_dir = std::fs::read_dir(&node_versions)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("v22."))
                .unwrap_or(false)
        })
        .next()?;
    let bin = v22_dir.join("installation").join("bin").join("openclaw");
    if bin.exists() { Some(bin) } else { None }
}

/// 将 fnm 中的 openclaw 软链到全局 PATH 可访问的位置。
/// 优先 /usr/local/bin，其次 ~/.local/bin。
#[cfg(not(target_os = "windows"))]
fn try_create_openclaw_symlink(app: &AppHandle, fnm_dir: &str, home_dir: &std::path::Path) {
    let src = match find_fnm_openclaw_binary(fnm_dir) {
        Some(p) => p,
        None => {
            emit_log(app, "⚠  未找到 fnm 目录中的 openclaw 二进制，请确认 npm 安装成功。");
            return;
        }
    };

    // 尝试 /usr/local/bin
    let dest1 = std::path::Path::new("/usr/local/bin/openclaw");
    let _ = std::fs::remove_file(dest1);
    if std::os::unix::fs::symlink(&src, dest1).is_ok() {
        emit_log(app, "已将 openclaw 软链到 /usr/local/bin/openclaw，终端可直接使用。");
        return;
    }

    // 回退 ~/.local/bin
    let local_bin = home_dir.join(".local").join("bin");
    let _ = std::fs::create_dir_all(&local_bin);
    let dest2 = local_bin.join("openclaw");
    let _ = std::fs::remove_file(&dest2);
    if std::os::unix::fs::symlink(&src, &dest2).is_ok() {
        emit_log(app, &format!(
            "已将 openclaw 软链到 {}。\n\
             若终端提示找不到命令，请在 ~/.zshrc 或 ~/.bash_profile 中添加：\n\
             export PATH=\"$HOME/.local/bin:$PATH\"",
            dest2.display()
        ));
        return;
    }

    emit_log(app, &format!(
        "⚠  无法自动将 openclaw 添加到 PATH。\n\
         二进制位于：{}\n\
         可手动执行：sudo ln -sf {} /usr/local/bin/openclaw",
        src.display(), src.display()
    ));
}

#[cfg(target_os = "windows")]
fn try_create_openclaw_symlink(_app: &AppHandle, _fnm_dir: &str, _home_dir: &std::path::Path) {
    // Windows: npm global bin 通常已在 PATH 中，无需额外处理
}

// ─── 安装标记文件 ──────────────────────────────────────────────────────────

/// 写入安装完成标记（openclaw npm 包已安装，但可能未 onboard）
fn write_installed_marker(app: &AppHandle) {
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("openclaw-npm-installed.flag"), "1");
    }
}

// ─── 核心安装流程 ──────────────────────────────────────────────────────────

async fn run_install_steps(
    app: &AppHandle,
    fnm_dir: &str,
    cancel_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    let home_dir = app.path().home_dir().map_err(|e| e.to_string())?;

    // ── 步骤 1：确保 Node.js ─────────────────────────────────────────────────
    let step1 = "install-node";
    let node_major = detect_system_node_major();
    let use_system_node = node_major.map_or(false, |v| v >= 22);

    if use_system_node {
        emit_step(app, step1, "running");
        emit_log(app, &format!(
            "检测到系统已安装 Node.js v{}（>= 22），跳过 fnm 安装。",
            node_major.unwrap()
        ));
        emit_step(app, step1, "done");
    } else {
        emit_step(app, step1, "running");
        if let Some(v) = node_major {
            emit_log(app, &format!(
                "检测到系统 Node.js v{} < 22，将通过 fnm 安装 Node.js 22...", v
            ));
        } else {
            emit_log(app, "未检测到系统 Node.js，将通过 fnm 安装 Node.js 22（首次下载约需 1~2 分钟）...");
        }
        match run_step(app, fnm_dir, &["install", "22"], cancel_rx).await {
            Ok(()) => emit_step(app, step1, "done"),
            Err(e) => {
                emit_step(app, step1, "error");
                emit_error(app, step1, &e);
                return Err(e);
            }
        }
    }

    // ── 步骤 2：npm install -g openclaw ──────────────────────────────────────
    let step2 = "install-openclaw";
    emit_step(app, step2, "running");
    emit_log(app, "正在通过 npm 安装 openclaw，请稍候...");

    let install_result = if use_system_node {
        run_login_shell_step(app, "npm install -g openclaw", cancel_rx).await
    } else {
        run_step(
            app, fnm_dir,
            &["exec", "--using=22", "--", "npm", "install", "-g", "openclaw"],
            cancel_rx,
        ).await
    };

    match install_result {
        Ok(()) => emit_step(app, step2, "done"),
        Err(e) => {
            emit_step(app, step2, "error");
            emit_error(app, step2, &e);
            return Err(e);
        }
    }

    // ── 安装后处理 ───────────────────────────────────────────────────────────
    // 若通过 fnm 安装，尝试将 openclaw 软链到全局 PATH
    if !use_system_node {
        try_create_openclaw_symlink(app, fnm_dir, &home_dir);
    }

    // 写入安装标记，供下次启动检测
    write_installed_marker(app);

    emit_log(app, "");
    emit_log(app, "OpenClaw 安装完成！");
    emit_log(app, "下一步：请在终端中运行 openclaw onboard 完成初始化配置。");

    Ok(())
}

// ─── Tauri Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_install(app: AppHandle) -> Result<(), String> {
    let state = app.try_state::<InstallerState>().ok_or("InstallerState not found")?;

    {
        let mut running = state.running.lock().unwrap();
        if *running {
            return Err("安装已在进行中".to_string());
        }
        *running = true;
    }

    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    { *state.cancel_tx.lock().unwrap() = Some(tx); }

    let fnm_dir = app
        .path()
        .app_data_dir()
        .map(|p| p.join("fnm").to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            format!("{home}/.local/share/claw-browser/fnm")
        });

    let app2 = app.clone();
    let running2 = state.running.clone();
    let cancel_tx2 = state.cancel_tx.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_install_steps(&app2, &fnm_dir, &mut rx).await;
        { *running2.lock().unwrap() = false; }
        { *cancel_tx2.lock().unwrap() = None; }

        match result {
            Ok(()) => {
                // npm 安装完成，通知前端引导用户手动运行 openclaw onboard
                let _ = app2.emit("installer:need-onboard", ());
            }
            Err(msg) => {
                if msg == "已取消" {
                    emit_log(&app2, "安装已取消。");
                }
            }
        }
    });

    Ok(())
}

/// OpenClaw 安装状态
#[derive(serde::Serialize)]
pub struct OpenclawInstallStatus {
    /// npm 包已安装（openclaw-npm-installed.flag 存在）
    pub npm_installed: bool,
    /// onboard 已完成（~/.openclaw/openclaw.json 存在）
    pub onboarded: bool,
}

/// 检测 OpenClaw 安装状态。
#[tauri::command]
pub fn check_openclaw_installed(app: AppHandle) -> OpenclawInstallStatus {
    let onboarded = app
        .path()
        .home_dir()
        .map(|h| h.join(".openclaw").join("openclaw.json").exists())
        .unwrap_or(false);

    let npm_installed = onboarded
        || app
            .path()
            .app_data_dir()
            .map(|d| d.join("openclaw-npm-installed.flag").exists())
            .unwrap_or(false);

    OpenclawInstallStatus { npm_installed, onboarded }
}

#[tauri::command]
pub async fn cancel_install(app: AppHandle) -> Result<(), String> {
    let state = app.try_state::<InstallerState>().ok_or("InstallerState not found")?;
    let mut guard = state.cancel_tx.lock().unwrap();
    if let Some(tx) = guard.take() {
        let _ = tx.send(());
    }
    Ok(())
}
