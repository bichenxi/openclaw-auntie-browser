//! OpenClaw 安装流程
//!
//! 检测优先级（从上到下，第一个满足的策略生效）：
//!
//! 1. 系统已有 nvm（~/.nvm/nvm.sh 存在）→ nvm install 22 + nvm alias default 22，再 nvm exec npm install
//! 2. 系统已有 fnm（登录 shell 可见）→ fnm install 22 + fnm default 22，再 fnm exec npm install
//! 3. 系统已有 node >= 22（无版本管理器）→ 直接用系统 npm install -g openclaw
//! 4. 以上均无                   → 使用 app 内置 fnm（独立隔离目录，不影响用户环境）
//!
//! 注意：
//! - 策略 1/2 会将 node 22 设为默认版本，确保新终端中 node/npm/openclaw 均可直接使用。
//! - 策略 1（nvm）由 nvm 自动管理 PATH，无需额外软链。
//! - 策略 2（fnm）安装后会尝试将 openclaw 软链到 /usr/local/bin 或 ~/.local/bin 作为备份。
//! - 策略 4（内置 fnm）会将 node bin 目录直接写入 shell profile（~/.zshrc），
//!   使 node、npm、openclaw 等命令在终端中全部可用。
//! - openclaw onboard 是交互式 TUI，无法在 app 内自动化，安装完成后由 UI 引导用户手动执行。

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
struct StepPayload { step: String, status: String }

#[derive(Clone, serde::Serialize)]
struct LogPayload { line: String }

#[derive(Clone, serde::Serialize)]
struct ErrorPayload { step: String, message: String }

fn emit_step(app: &AppHandle, step: &str, status: &str) {
    let _ = app.emit("installer:step", StepPayload { step: step.into(), status: status.into() });
}
fn emit_log(app: &AppHandle, line: &str) {
    let _ = app.emit("installer:log", LogPayload { line: line.into() });
}
fn emit_error(app: &AppHandle, step: &str, message: &str) {
    let _ = app.emit("installer:error", ErrorPayload { step: step.into(), message: message.into() });
}

// ─── 环境检测 ──────────────────────────────────────────────────────────────

/// Node.js 安装策略
#[derive(Debug)]
enum NodeStrategy {
    /// 系统已有 node >= 22，直接用系统 npm
    SystemNode(u32),
    /// 系统已安装 fnm（登录 shell 可见），用系统 fnm 管理 node 22
    SystemFnm,
    /// 系统已安装 nvm（~/.nvm/nvm.sh 存在），用 nvm 管理 node 22
    SystemNvm,
    /// 无版本管理工具，使用 app 内置 fnm（独立隔离目录）
    BundledFnm,
}

/// 返回最合适的登录 shell（$SHELL → zsh → bash → sh）
fn detect_login_shell() -> String {
    if let Ok(s) = std::env::var("SHELL") {
        if std::path::Path::new(&s).exists() { return s; }
    }
    for sh in &["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if std::path::Path::new(sh).exists() { return sh.to_string(); }
    }
    "/bin/sh".to_string()
}

/// 通过登录 shell 检测 node major 版本，返回 None 表示未安装或无法检测。
fn detect_system_node_major() -> Option<u32> {
    let shell = detect_login_shell();
    let out = std::process::Command::new(&shell)
        .args(["-l", "-c", "node --version"])
        .output().ok()?;
    if !out.status.success() { return None; }
    let s = String::from_utf8_lossy(&out.stdout);
    let trimmed = s.trim().trim_start_matches('v');
    trimmed.split('.').next()?.parse::<u32>().ok()
}

/// 通过登录 shell 检测 fnm 是否可用（binary 在 PATH 中）
fn detect_system_fnm() -> bool {
    let shell = detect_login_shell();
    std::process::Command::new(&shell)
        .args(["-l", "-c", "fnm --version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 综合检测，返回最合适的安装策略。
/// 优先使用版本管理器（nvm/fnm），确保安装后能设为默认版本。
fn detect_node_strategy(home_dir: &std::path::Path) -> NodeStrategy {
    // 1. 系统 nvm？优先，安装 node 22 并设为默认
    if home_dir.join(".nvm").join("nvm.sh").exists() {
        return NodeStrategy::SystemNvm;
    }
    // 2. 系统 fnm？
    if detect_system_fnm() {
        return NodeStrategy::SystemFnm;
    }
    // 3. 无版本管理器，系统已有 node >= 22
    if let Some(major) = detect_system_node_major() {
        if major >= 22 {
            return NodeStrategy::SystemNode(major);
        }
    }
    // 4. 兜底：内置 fnm
    NodeStrategy::BundledFnm
}

// ─── 命令执行（流式输出）─────────────────────────────────────────────────────

/// npm notice / npm warn EBADENGINE 等纯噪音行，不向前端显示。
fn is_npm_noise(line: &str) -> bool {
    let l = line.trim();
    l.starts_with("npm notice") || l.starts_with("npm warn EBADENGINE")
}

/// 通过登录 shell 运行命令，实时推送 stdout/stderr 到前端。
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
            .stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().map_err(|e| format!("启动命令失败：{}", e))?
    };

    #[cfg(target_os = "windows")]
    let mut child = Command::new("cmd")
        .args(["/C", cmd_str])
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|e| format!("启动命令失败：{}", e))?;

    let mut out = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut err = BufReader::new(child.stderr.take().unwrap()).lines();
    let (mut out_done, mut err_done) = (false, false);

    loop {
        if out_done && err_done { break; }
        tokio::select! {
            _ = &mut *cancel_rx => {
                let _ = child.kill().await;
                return Err("已取消".to_string());
            }
            line = out.next_line(), if !out_done => match line {
                Ok(Some(l)) => { if !is_npm_noise(&l) { emit_log(app, &l); } }
                _ => out_done = true,
            },
            line = err.next_line(), if !err_done => match line {
                Ok(Some(l)) => { if !is_npm_noise(&l) { emit_log(app, &l); } }
                _ => err_done = true,
            },
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() { Ok(()) }
    else { Err(format!("进程退出码 {}", status.code().unwrap_or(-1))) }
}

/// 用 app 内置 fnm sidecar 运行命令，实时推送 stdout/stderr 到前端。
async fn run_step(
    app: &AppHandle,
    fnm_dir: &str,
    args: &[&str],
    cancel_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    use tauri_plugin_shell::process::CommandEvent;

    let mut cmd = app.shell().sidecar("fnm").map_err(|e| e.to_string())?;
    cmd = cmd.args(["--fnm-dir", fnm_dir]).args(args);
    let (mut rx, child) = cmd.spawn().map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            _ = &mut *cancel_rx => {
                let _ = child.kill();
                return Err("已取消".to_string());
            }
            ev = rx.recv() => match ev {
                Some(CommandEvent::Stdout(b)) => {
                    let l = String::from_utf8_lossy(&b);
                    if !is_npm_noise(l.trim_end()) { emit_log(app, l.trim_end()); }
                }
                Some(CommandEvent::Stderr(b)) => {
                    let l = String::from_utf8_lossy(&b);
                    if !is_npm_noise(l.trim_end()) { emit_log(app, l.trim_end()); }
                }
                Some(CommandEvent::Terminated(p)) => {
                    let code = p.code.unwrap_or(-1);
                    return if code == 0 { Ok(()) } else { Err(format!("进程退出码 {}", code)) };
                }
                Some(_) => {}
                None => return Ok(()),
            }
        }
    }
}

// ─── 软链辅助 ──────────────────────────────────────────────────────────────

/// 当 openclaw 被软链到 ~/.local/bin 时，自动将该目录加入 shell profile 的 PATH。
#[cfg(not(target_os = "windows"))]
fn ensure_local_bin_in_path(app: &AppHandle, home_dir: &std::path::Path) {
    use std::io::Write;

    let export_line = r#"export PATH="$HOME/.local/bin:$PATH""#;
    let shell = detect_login_shell();
    let profile_name = if shell.contains("zsh") { ".zshrc" } else { ".bash_profile" };
    let profile = home_dir.join(profile_name);

    if let Ok(content) = std::fs::read_to_string(&profile) {
        if content.contains(".local/bin") {
            return;
        }
    }

    match std::fs::OpenOptions::new().append(true).create(true).open(&profile) {
        Ok(mut f) => {
            if writeln!(f, "\n# Added by Claw Browser\n{}", export_line).is_ok() {
                emit_log(app, &format!(
                    "已自动将 ~/.local/bin 添加到 ~/{} 的 PATH 中。",
                    profile_name
                ));
                emit_log(app, "⚠ 请关闭并重新打开终端窗口，使 PATH 配置生效后再执行 openclaw 命令。");
            }
        }
        Err(_) => {
            emit_log(app, &format!(
                "⚠ 无法写入 ~/{}，请手动添加：\n{}",
                profile_name, export_line
            ));
        }
    }
}

/// 将 src 软链到全局 PATH 目录：
/// 1. /usr/local/bin（Intel Mac / Homebrew 用户通常有写权限）
/// 2. /opt/homebrew/bin（Apple Silicon Mac + Homebrew）
/// 3. ~/.local/bin（兜底，同时自动写入 shell profile）
#[cfg(not(target_os = "windows"))]
fn do_symlink(app: &AppHandle, src: &std::path::Path, home_dir: &std::path::Path) {
    let dest1 = std::path::Path::new("/usr/local/bin/openclaw");
    let _ = std::fs::remove_file(dest1);
    if std::os::unix::fs::symlink(src, dest1).is_ok() {
        emit_log(app, "已将 openclaw 软链到 /usr/local/bin/openclaw");
        return;
    }

    let homebrew_bin = std::path::Path::new("/opt/homebrew/bin");
    if homebrew_bin.is_dir() {
        let dest2 = homebrew_bin.join("openclaw");
        let _ = std::fs::remove_file(&dest2);
        if std::os::unix::fs::symlink(src, &dest2).is_ok() {
            emit_log(app, &format!("已将 openclaw 软链到 {}", dest2.display()));
            return;
        }
    }

    let local_bin = home_dir.join(".local").join("bin");
    let _ = std::fs::create_dir_all(&local_bin);
    let dest3 = local_bin.join("openclaw");
    let _ = std::fs::remove_file(&dest3);
    if std::os::unix::fs::symlink(src, &dest3).is_ok() {
        emit_log(app, &format!("已将 openclaw 软链到 {}", dest3.display()));
        ensure_local_bin_in_path(app, home_dir);
        return;
    }

    emit_log(app, &format!(
        "⚠ 无法自动添加到 PATH，openclaw 位于：{}\n\
         可手动执行：sudo ln -sf '{}' /usr/local/bin/openclaw",
        src.display(), src.display()
    ));
}

/// 用 shell 执行 which_cmd 找到 openclaw 路径，若不在全局 PATH 则尝试软链。
#[cfg(not(target_os = "windows"))]
fn try_symlink_from_which(app: &AppHandle, which_cmd: &str, home_dir: &std::path::Path) {
    let shell = detect_login_shell();
    let output = std::process::Command::new(&shell)
        .args(["-l", "-c", which_cmd])
        .output();

    let src = match output {
        Ok(o) if o.status.success() => {
            let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if p.is_empty() { None } else { Some(std::path::PathBuf::from(p)) }
        }
        _ => None,
    };

    match src {
        None => {
            emit_log(app, "⚠ 无法定位 openclaw 二进制，请确认安装成功后手动添加到 PATH。");
        }
        Some(path) => {
            // 已在常见全局 PATH 目录，无需软链
            let is_global = ["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"]
                .iter()
                .any(|p| path.starts_with(p));
            if is_global {
                emit_log(app, &format!("openclaw 已在 {}，终端可直接使用。", path.display()));
            } else {
                emit_log(app, &format!("openclaw 位于 {}，尝试软链到全局 PATH...", path.display()));
                do_symlink(app, &path, home_dir);
            }
        }
    }
}

/// BundledFnm 安装后处理：找到内置 fnm 的 node bin 目录，
/// 将其直接写入 shell profile（~/.zshrc 或 ~/.bash_profile），
/// 使 node、npm、npx、openclaw 等命令在终端中全部可用。
#[cfg(not(target_os = "windows"))]
fn try_symlink_bundled_fnm(app: &AppHandle, fnm_dir: &str, home_dir: &std::path::Path) {
    let node_versions = std::path::Path::new(fnm_dir).join("node-versions");
    let bin_dir = std::fs::read_dir(&node_versions)
        .ok()
        .and_then(|mut rd| {
            rd.find(|e| {
                e.as_ref().ok()
                    .and_then(|e| e.file_name().into_string().ok())
                    .map(|n| n.starts_with("v22."))
                    .unwrap_or(false)
            })
        })
        .and_then(|e| e.ok())
        .map(|e| e.path().join("installation").join("bin"))
        .filter(|p| p.join("openclaw").exists());

    match bin_dir {
        Some(bin) => {
            emit_log(app, &format!(
                "内置 fnm Node.js 位于 {}，正在添加到终端 PATH...",
                bin.display()
            ));
            add_node_bin_to_shell_profile(app, &bin, home_dir);
        }
        None => {
            emit_log(app, "⚠ 未在内置 fnm 目录中找到 openclaw 二进制，请确认 npm 安装成功。");
        }
    }
}

/// 将 node bin 目录写入 shell profile，使 node/npm/openclaw 全部可用。
#[cfg(not(target_os = "windows"))]
fn add_node_bin_to_shell_profile(app: &AppHandle, node_bin_dir: &std::path::Path, home_dir: &std::path::Path) {
    use std::io::Write;

    let bin_path_str = node_bin_dir.display().to_string();
    let export_line = format!(r#"export PATH="{}:$PATH""#, bin_path_str);

    let shell = detect_login_shell();
    let profile_name = if shell.contains("zsh") { ".zshrc" } else { ".bash_profile" };
    let profile = home_dir.join(profile_name);

    if let Ok(content) = std::fs::read_to_string(&profile) {
        if content.contains(&bin_path_str) {
            emit_log(app, &format!(
                "~/{} 已包含内置 Node.js PATH 配置，跳过。", profile_name
            ));
            return;
        }
    }

    match std::fs::OpenOptions::new().append(true).create(true).open(&profile) {
        Ok(mut f) => {
            if writeln!(f, "\n# Added by Claw Browser — bundled Node.js\n{}", export_line).is_ok() {
                emit_log(app, &format!(
                    "已将内置 Node.js 路径添加到 ~/{} 中。", profile_name
                ));
                emit_log(app, "重新打开终端后 node、npm、openclaw 等命令均可使用。");
            }
        }
        Err(_) => {
            emit_log(app, &format!(
                "⚠ 无法写入 ~/{}，请手动添加以下内容：\n{}", profile_name, export_line
            ));
        }
    }
}

#[cfg(target_os = "windows")]
fn try_symlink_from_which(_: &AppHandle, _: &str, _: &std::path::Path) {}
#[cfg(target_os = "windows")]
fn try_symlink_bundled_fnm(_: &AppHandle, _: &str, _: &std::path::Path) {}

// ─── 安装标记文件 ──────────────────────────────────────────────────────────

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
    let nvm_sh = home_dir.join(".nvm").join("nvm.sh");

    let strategy = detect_node_strategy(&home_dir);

    // ── 检测结果日志 ──────────────────────────────────────────────────────────
    match &strategy {
        NodeStrategy::SystemNode(v) =>
            emit_log(app, &format!("检测到系统 Node.js v{}（>= 22），将直接使用系统 npm 安装。", v)),
        NodeStrategy::SystemFnm =>
            emit_log(app, "检测到系统已安装 fnm，将使用系统 fnm 安装 Node.js 22 并设为默认版本。"),
        NodeStrategy::SystemNvm =>
            emit_log(app, "检测到系统已安装 nvm，将使用系统 nvm 安装 Node.js 22 并设为默认版本。"),
        NodeStrategy::BundledFnm => {
            emit_log(app, "未检测到系统 node 版本管理工具，将使用应用内置 fnm（独立隔离目录）安装 Node.js 22。");
            emit_log(app, &format!("内置 fnm 目录：{}", fnm_dir));
            emit_log(app, "注意：此模式下在终端执行 `fnm ls` 不会显示这里安装的 node 版本，这是正常现象。");
        }
    }

    // ── 步骤 1：确保 Node.js 22 可用 ─────────────────────────────────────────
    let step1 = "install-node";
    emit_step(app, step1, "running");

    match &strategy {
        NodeStrategy::SystemNode(v) => {
            emit_log(app, &format!("系统 Node.js v{} 满足要求（>= 22），跳过安装。", v));
            emit_step(app, step1, "done");
        }
        NodeStrategy::SystemFnm => {
            emit_log(app, "正在通过系统 fnm 安装 Node.js 22 并设为默认版本...");
            match run_login_shell_step(app, "fnm install 22 && fnm default 22", cancel_rx).await {
                Ok(()) => emit_step(app, step1, "done"),
                Err(e) => { emit_step(app, step1, "error"); emit_error(app, step1, &e); return Err(e); }
            }
        }
        NodeStrategy::SystemNvm => {
            emit_log(app, "正在通过系统 nvm 安装 Node.js 22 并设为默认版本...");
            let cmd = format!("source '{}' && nvm install 22 && nvm alias default 22", nvm_sh.display());
            match run_login_shell_step(app, &cmd, cancel_rx).await {
                Ok(()) => emit_step(app, step1, "done"),
                Err(e) => { emit_step(app, step1, "error"); emit_error(app, step1, &e); return Err(e); }
            }
        }
        NodeStrategy::BundledFnm => {
            emit_log(app, "正在通过内置 fnm 安装 Node.js 22（首次下载约需 1~2 分钟）...");
            match run_step(app, fnm_dir, &["install", "22"], cancel_rx).await {
                Ok(()) => emit_step(app, step1, "done"),
                Err(e) => { emit_step(app, step1, "error"); emit_error(app, step1, &e); return Err(e); }
            }
        }
    }

    // ── 步骤 2：npm install -g openclaw ──────────────────────────────────────
    let step2 = "install-openclaw";
    emit_step(app, step2, "running");
    emit_log(app, "正在通过 npm 全局安装 openclaw，请稍候...");

    let install_result = match &strategy {
        NodeStrategy::SystemNode(_) =>
            run_login_shell_step(app, "npm install -g openclaw --no-update-notifier --no-fund", cancel_rx).await,
        NodeStrategy::SystemFnm =>
            run_login_shell_step(app, "fnm exec --using=22 -- npm install -g openclaw --no-update-notifier --no-fund", cancel_rx).await,
        NodeStrategy::SystemNvm => {
            let cmd = format!("source '{}' && nvm exec 22 npm install -g openclaw --no-update-notifier --no-fund", nvm_sh.display());
            run_login_shell_step(app, &cmd, cancel_rx).await
        }
        NodeStrategy::BundledFnm =>
            run_step(app, fnm_dir, &["exec", "--using=22", "--", "npm", "install", "-g", "openclaw", "--no-update-notifier", "--no-fund"], cancel_rx).await,
    };

    match install_result {
        Ok(()) => emit_step(app, step2, "done"),
        Err(e) => {
            emit_step(app, step2, "error");
            emit_log(app, "");
            emit_log(app, "npm 安装失败，常见原因及解决方法：");
            emit_log(app, "  1. 网络问题（大陆用户）：可尝试切换 npm 镜像源后重试");
            emit_log(app, "     npm config set registry https://registry.npmmirror.com");
            emit_log(app, "  2. 权限问题：尝试在终端手动执行 npm install -g openclaw");
            emit_log(app, "  3. 详细日志见 ~/.npm/_logs/ 目录下最新的 debug 文件");
            emit_error(app, step2, &e);
            return Err(e);
        }
    }

    // ── 安装后处理：确保 openclaw 在终端 PATH 中 ─────────────────────────────
    match &strategy {
        NodeStrategy::SystemNode(_) => {
            try_symlink_from_which(app, "which openclaw", &home_dir);
        }
        NodeStrategy::SystemFnm => {
            try_symlink_from_which(app, "fnm exec --using=22 -- which openclaw", &home_dir);
        }
        NodeStrategy::SystemNvm => {
            emit_log(app, "nvm 已将 Node.js 22 设为默认版本，新终端中 node/npm/openclaw 均可直接使用。");
        }
        NodeStrategy::BundledFnm => {
            try_symlink_bundled_fnm(app, fnm_dir, &home_dir);
        }
    }

    write_installed_marker(app);

    // ── 最终验证：在登录 shell 中确认 openclaw 可访问 ──────────────────────
    let shell = detect_login_shell();
    let verified = std::process::Command::new(&shell)
        .args(["-l", "-c", "openclaw --version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    emit_log(app, "");
    emit_log(app, "OpenClaw 安装完成！");
    if verified {
        emit_log(app, "已验证 openclaw 命令可用。");
    } else {
        emit_log(app, "⚠ 当前终端环境尚未识别 openclaw 命令。");
        emit_log(app, "请关闭并重新打开终端窗口（使 PATH 生效），然后执行：");
    }
    emit_log(app, "下一步：请在终端中运行 openclaw onboard 完成初始化配置。");

    Ok(())
}

// ─── Tauri Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_install(app: AppHandle) -> Result<(), String> {
    let state = app.try_state::<InstallerState>().ok_or("InstallerState not found")?;

    {
        let mut running = state.running.lock().unwrap();
        if *running { return Err("安装已在进行中".to_string()); }
        *running = true;
    }

    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    { *state.cancel_tx.lock().unwrap() = Some(tx); }

    let fnm_dir = app.path().app_data_dir()
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
            Ok(()) => { let _ = app2.emit("installer:need-onboard", ()); }
            Err(msg) => { if msg == "已取消" { emit_log(&app2, "安装已取消。"); } }
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

#[tauri::command]
pub fn check_openclaw_installed(app: AppHandle) -> OpenclawInstallStatus {
    let onboarded = app.path().home_dir()
        .map(|h| h.join(".openclaw").join("openclaw.json").exists())
        .unwrap_or(false);

    let npm_installed = onboarded || app.path().app_data_dir()
        .map(|d| d.join("openclaw-npm-installed.flag").exists())
        .unwrap_or(false);

    OpenclawInstallStatus { npm_installed, onboarded }
}

#[tauri::command]
pub async fn cancel_install(app: AppHandle) -> Result<(), String> {
    let state = app.try_state::<InstallerState>().ok_or("InstallerState not found")?;
    let mut guard = state.cancel_tx.lock().unwrap();
    if let Some(tx) = guard.take() { let _ = tx.send(()); }
    Ok(())
}
