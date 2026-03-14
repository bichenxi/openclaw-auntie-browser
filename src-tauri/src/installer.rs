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

/// npmmirror Node.js 发行版镜像，解决大陆用户从 nodejs.org 下载失败的问题。
const NODE_DIST_MIRROR: &str = "https://npmmirror.com/mirrors/node";

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
    /// 系统已安装 nvm，用 nvm 管理 node 22
    /// Unix: ~/.nvm/nvm.sh 存在；Windows: nvm-windows（%APPDATA%\nvm\nvm.exe）
    SystemNvm,
    /// 无版本管理工具，使用 app 内置 fnm（独立隔离目录）
    BundledFnm,
}

/// 返回最合适的登录 shell。
/// Unix: $SHELL → zsh → bash → sh；Windows: %COMSPEC% → cmd.exe
pub(crate) fn detect_login_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(s) = std::env::var("SHELL") {
            if std::path::Path::new(&s).exists() { return s; }
        }
        for sh in &["/bin/zsh", "/bin/bash", "/bin/sh"] {
            if std::path::Path::new(sh).exists() { return sh.to_string(); }
        }
        "/bin/sh".to_string()
    }
}

/// 通过系统 shell 运行单条命令并返回 Output（跨平台）。
/// Unix: 用登录 shell `-l -c`；Windows: 用 `cmd /C`。
pub(crate) fn run_in_shell(cmd: &str) -> std::io::Result<std::process::Output> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/C", cmd]).output()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let shell = detect_login_shell();
        std::process::Command::new(&shell).args(["-l", "-c", cmd]).output()
    }
}

/// 检测 node major 版本，返回 None 表示未安装或无法检测。
fn detect_system_node_major() -> Option<u32> {
    let out = run_in_shell("node --version").ok()?;
    if !out.status.success() { return None; }
    let s = String::from_utf8_lossy(&out.stdout);
    let trimmed = s.trim().trim_start_matches('v');
    trimmed.split('.').next()?.parse::<u32>().ok()
}

/// 检测 fnm 是否可用（binary 在 PATH 中）
fn detect_system_fnm() -> bool {
    run_in_shell("fnm --version")
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 综合检测，返回最合适的安装策略。
///
/// Unix: 优先版本管理器（nvm > fnm），确保安装后能设为默认版本。
/// Windows: 优先已有 node（无权限问题），版本管理器次之。
fn detect_node_strategy(home_dir: &std::path::Path) -> NodeStrategy {
    let _ = home_dir; // used only on Unix; suppress warning on Windows

    // Windows: 先检查已有 node，避免 fnm/nvm 的权限问题
    #[cfg(target_os = "windows")]
    {
        if let Some(major) = detect_system_node_major() {
            if major >= 22 {
                return NodeStrategy::SystemNode(major);
            }
        }
    }

    // 系统 nvm？
    //   Unix: ~/.nvm/nvm.sh 存在
    //   Windows: nvm-windows（%APPDATA%\nvm\nvm.exe 或 PATH 中）
    #[cfg(not(target_os = "windows"))]
    let has_nvm = home_dir.join(".nvm").join("nvm.sh").exists();
    #[cfg(target_os = "windows")]
    let has_nvm = {
        let nvm_exe = std::env::var("APPDATA")
            .map(|a| std::path::PathBuf::from(a).join("nvm").join("nvm.exe"))
            .unwrap_or_default();
        nvm_exe.exists()
            || run_in_shell("nvm version").map(|o| o.status.success()).unwrap_or(false)
    };
    if has_nvm {
        return NodeStrategy::SystemNvm;
    }
    // 系统 fnm？
    if detect_system_fnm() {
        return NodeStrategy::SystemFnm;
    }
    // Unix: 系统已有 node >= 22（Windows 已在前面检测过）
    #[cfg(not(target_os = "windows"))]
    if let Some(major) = detect_system_node_major() {
        if major >= 22 {
            return NodeStrategy::SystemNode(major);
        }
    }
    // 兜底：内置 fnm
    NodeStrategy::BundledFnm
}

// ─── Windows 工具路径补全 ────────────────────────────────────────────────────

/// 运行时发现/下载的 Git 目录，跨线程共享。
#[cfg(target_os = "windows")]
static DISCOVERED_GIT_DIR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Windows: 返回包含 git 的增强 PATH。
/// 不依赖 `std::env::set_var`，而是通过 `DISCOVERED_GIT_DIR` + `find_git_cmd_dir` 拼装。
#[cfg(target_os = "windows")]
fn augmented_path_with_git() -> Option<String> {
    let current = std::env::var("PATH").unwrap_or_default();

    if let Some(dir) = DISCOVERED_GIT_DIR.get() {
        if !current.contains(dir.as_str()) {
            return Some(format!("{};{}", dir, current));
        }
    }

    if std::process::Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return None;
    }

    if let Some(dir) = find_git_cmd_dir() {
        return Some(format!("{};{}", dir, current));
    }
    None
}

/// Windows: 搜索 Git for Windows 的 cmd 目录，覆盖主流安装方式。
#[cfg(target_os = "windows")]
fn find_git_cmd_dir() -> Option<String> {
    let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let pf86 = std::env::var("PROGRAMFILES(X86)")
        .unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let userprofile = std::env::var("USERPROFILE").unwrap_or_default();

    let candidates = [
        format!(r"{}\Git\cmd", pf),
        format!(r"{}\Git\cmd", pf86),
        format!(r"{}\Programs\Git\cmd", local),
        format!(r"{}\scoop\shims", userprofile),
        format!(r"{}\scoop\apps\git\current\cmd", userprofile),
    ];

    for dir in &candidates {
        if std::path::Path::new(dir).join("git.exe").exists() {
            return Some(dir.clone());
        }
    }

    // GitHub Desktop 捆绑的 PortableGit
    let gh_git_base = std::path::Path::new(&local).join("GitHub");
    if let Ok(rd) = std::fs::read_dir(&gh_git_base) {
        for e in rd.flatten() {
            let name = e.file_name();
            if name.to_string_lossy().starts_with("PortableGit") {
                let cmd_dir = e.path().join("cmd");
                if cmd_dir.join("git.exe").exists() {
                    return Some(cmd_dir.to_string_lossy().to_string());
                }
            }
        }
    }

    // Oclaw 下载的便携版 MinGit
    if let Ok(appdata) = std::env::var("APPDATA") {
        let mingit_cmd = std::path::Path::new(&appdata)
            .join("com.claw.browser").join("mingit").join("cmd");
        if mingit_cmd.join("git.exe").exists() {
            return Some(mingit_cmd.to_string_lossy().to_string());
        }
    }
    if let Ok(local_ad) = std::env::var("LOCALAPPDATA") {
        let mingit_cmd = std::path::Path::new(&local_ad)
            .join("com.claw.browser").join("mingit").join("cmd");
        if mingit_cmd.join("git.exe").exists() {
            return Some(mingit_cmd.to_string_lossy().to_string());
        }
    }

    None
}

/// Windows: 下载便携版 MinGit 到应用数据目录，无需管理员权限。
/// 成功返回 MinGit 的 cmd 目录路径。
#[cfg(target_os = "windows")]
fn download_portable_git(app: &AppHandle) -> Option<String> {
    let app_data = app.path().app_data_dir().ok()?;
    let mingit_dir = app_data.join("mingit");
    let git_cmd_dir = mingit_dir.join("cmd");

    if git_cmd_dir.join("git.exe").exists() {
        return Some(git_cmd_dir.to_string_lossy().to_string());
    }

    emit_log(app, "正在下载便携版 Git（MinGit，约 35MB，无需管理员权限）...");

    let zip_path = app_data.join("mingit.zip");
    let _ = std::fs::create_dir_all(&app_data);

    // 从多个版本 × 多个镜像中逐个尝试，提高成功率
    let versions: &[(&str, &str)] = &[
        ("2.47.1.windows.1", "2.47.1"),
        ("2.48.1.windows.1", "2.48.1"),
        ("2.46.2.windows.1", "2.46.2"),
    ];

    let mut urls: Vec<(String, &str)> = Vec::new();
    for (tag, ver) in versions {
        urls.push((
            format!("https://registry.npmmirror.com/-/binary/git-for-windows/v{}/MinGit-{}-64-bit.zip", tag, ver),
            "npmmirror 镜像",
        ));
        urls.push((
            format!("https://mirrors.huaweicloud.com/git-for-windows/v{}/MinGit-{}-64-bit.zip", tag, ver),
            "华为云镜像",
        ));
        urls.push((
            format!("https://github.com/git-for-windows/git/releases/download/v{}/MinGit-{}-64-bit.zip", tag, ver),
            "GitHub",
        ));
    }

    let mut downloaded = false;
    for (url, source) in &urls {
        emit_log(app, &format!("  尝试 {}...", source));
        let zip_str = zip_path.to_string_lossy().to_string();

        let curl_result = std::process::Command::new("curl.exe")
            .args(["-L", "--fail", "--silent", "--show-error",
                   "--connect-timeout", "15", "--max-time", "300",
                   "-o", &zip_str, url.as_str()])
            .output();
        match &curl_result {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                let msg = String::from_utf8_lossy(&o.stderr);
                if !msg.trim().is_empty() {
                    emit_log(app, &format!("    curl 失败：{}", msg.trim()));
                }
            }
            Err(e) => emit_log(app, &format!("    curl 不可用：{}", e)),
        }
        if zip_path.exists() && std::fs::metadata(&zip_path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
            downloaded = true;
            emit_log(app, &format!("  ✓ 从 {} 下载成功", source));
            break;
        }
        let _ = std::fs::remove_file(&zip_path);

        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol=[Net.SecurityProtocolType]::Tls12;\
             Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            url, zip_str
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .output();
        if zip_path.exists() && std::fs::metadata(&zip_path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
            downloaded = true;
            emit_log(app, &format!("  ✓ 从 {} 下载成功（PowerShell）", source));
            break;
        }
        let _ = std::fs::remove_file(&zip_path);
    }

    if !downloaded {
        emit_log(app, "所有下载源均失败，请检查网络连接。");
        return None;
    }

    emit_log(app, "下载完成，正在解压...");
    let _ = std::fs::create_dir_all(&mingit_dir);
    let extract_ok = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!(
            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            zip_path.to_string_lossy(), mingit_dir.to_string_lossy()
        )])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = std::fs::remove_file(&zip_path);

    if extract_ok && git_cmd_dir.join("git.exe").exists() {
        emit_log(app, "✓ 便携版 Git 安装成功！");
        Some(git_cmd_dir.to_string_lossy().to_string())
    } else {
        emit_log(app, "解压失败。");
        let _ = std::fs::remove_dir_all(&mingit_dir);
        None
    }
}

/// 记录 git 所在目录到 DISCOVERED_GIT_DIR，供后续子进程使用。
#[cfg(target_os = "windows")]
fn remember_git_dir(dir: &str) {
    let _ = DISCOVERED_GIT_DIR.set(dir.to_string());
}

/// 写入 ~/.npmrc 的 git 配置，使 npm 即使 PATH 缺失也能找到 git。
/// 直接操作文件比调用 `npm config set` 更可靠（此时 npm 可能不在 PATH 中）。
#[cfg(target_os = "windows")]
fn configure_npm_git(app: &AppHandle, git_exe: &str) {
    use std::io::Write;
    let git_path_forward = git_exe.replace('\\', "/");
    let line = format!("git={}", git_path_forward);

    let home = std::env::var("USERPROFILE").unwrap_or_default();
    if home.is_empty() { return; }
    let npmrc = std::path::Path::new(&home).join(".npmrc");

    let existing = std::fs::read_to_string(&npmrc).unwrap_or_default();
    if existing.contains("git=") {
        let updated: String = existing
            .lines()
            .map(|l| if l.starts_with("git=") { line.as_str() } else { l })
            .collect::<Vec<_>>()
            .join("\n");
        if std::fs::write(&npmrc, format!("{}\n", updated.trim_end())).is_ok() {
            emit_log(app, &format!("已更新 ~/.npmrc: {}", line));
        }
    } else {
        match std::fs::OpenOptions::new().append(true).create(true).open(&npmrc) {
            Ok(mut f) => {
                if writeln!(f, "{}", line).is_ok() {
                    emit_log(app, &format!("已写入 ~/.npmrc: {}", line));
                }
            }
            Err(e) => emit_log(app, &format!("写入 ~/.npmrc 失败：{}", e)),
        }
    }
}

/// Windows: 检测 Git 是否可用，不可用时依次尝试：
/// 1. winget 安装（需要管理员权限）
/// 2. 下载便携版 MinGit（无需管理员权限）
/// 3. 提示手动安装
/// 返回 true 表示 git 已可用。
#[cfg(target_os = "windows")]
fn ensure_git_available(app: &AppHandle) -> bool {
    // 已经通过 PATH 找到 git
    if std::process::Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }

    // 在已知路径中搜索 git（含之前下载的 MinGit）
    if let Some(dir) = find_git_cmd_dir() {
        let git_exe = format!(r"{}\git.exe", dir);
        emit_log(app, &format!("在 {} 发现 Git", dir));
        remember_git_dir(&dir);
        configure_npm_git(app, &git_exe);
        return true;
    }

    // 检查 Tauri app data 目录下已有的 MinGit
    if let Ok(app_data) = app.path().app_data_dir() {
        let mingit_cmd = app_data.join("mingit").join("cmd");
        if mingit_cmd.join("git.exe").exists() {
            let dir = mingit_cmd.to_string_lossy().to_string();
            let git_exe = mingit_cmd.join("git.exe").to_string_lossy().to_string();
            emit_log(app, &format!("发现已下载的 MinGit：{}", dir));
            remember_git_dir(&dir);
            configure_npm_git(app, &git_exe);
            return true;
        }
    }

    emit_log(app, "");
    emit_log(app, "⚠ 未检测到 Git，openclaw 的部分依赖需要 git 来安装。");

    // 策略 1: winget
    emit_log(app, "正在尝试通过 winget 自动安装 Git for Windows...");
    let winget_ok = std::process::Command::new("winget")
        .args(["install", "-e", "--id", "Git.Git", "--silent",
               "--accept-package-agreements", "--accept-source-agreements"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if winget_ok {
        emit_log(app, "✓ Git for Windows 安装成功！");
        if let Some(dir) = find_git_cmd_dir() {
            let git_exe = format!(r"{}\git.exe", dir);
            remember_git_dir(&dir);
            configure_npm_git(app, &git_exe);
            return true;
        }
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".to_string());
        let default_git = format!(r"{}\Git\cmd\git.exe", pf);
        if std::path::Path::new(&default_git).exists() {
            let dir = format!(r"{}\Git\cmd", pf);
            remember_git_dir(&dir);
            configure_npm_git(app, &default_git);
            return true;
        }
    } else {
        emit_log(app, "winget 自动安装失败（可能系统无 winget 或需要管理员权限）。");
    }

    // 策略 2: 下载便携版 MinGit
    emit_log(app, "正在尝试下载便携版 MinGit（无需管理员权限）...");
    if let Some(git_dir) = download_portable_git(app) {
        let git_exe = format!(r"{}\git.exe", git_dir);
        remember_git_dir(&git_dir);
        configure_npm_git(app, &git_exe);
        return true;
    }

    // 策略 3: 手动安装指引
    emit_log(app, "");
    emit_log(app, "自动安装均失败，请手动安装 Git for Windows：");
    emit_log(app, "  下载地址：https://git-scm.com/download/win");
    emit_log(app, "  安装时请勾选 \"Add to PATH\"");
    emit_log(app, "  安装完成后重新点击安装按钮即可。");
    false
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
    let mut child = {
        let mut b = Command::new("cmd");
        b.args(["/C", cmd_str]).stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(path) = augmented_path_with_git() {
            b.env("PATH", path);
        }
        b.spawn().map_err(|e| format!("启动命令失败：{}", e))?
    };

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
    cmd = cmd.args(["--fnm-dir", fnm_dir, "--node-dist-mirror", NODE_DIST_MIRROR]).args(args);
    #[cfg(target_os = "windows")]
    if let Some(path) = augmented_path_with_git() {
        cmd = cmd.env("PATH", path);
    }
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
            if writeln!(f, "\n# Added by Oclaw\n{}", export_line).is_ok() {
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
            if writeln!(f, "\n# Added by Oclaw — bundled Node.js\n{}", export_line).is_ok() {
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

/// Windows: 通过 PowerShell 将目录列表追加到用户 PATH（不影响系统 PATH）。
/// 仅追加当前 PATH 中尚未包含的目录，避免重复。
#[cfg(target_os = "windows")]
fn try_add_to_user_path_windows(app: &AppHandle, dirs: &[String]) -> bool {
    let ps_script = format!(
        r#"$p = [Environment]::GetEnvironmentVariable('PATH','User'); $toAdd = @({dirs_quoted}); foreach($d in $toAdd){{ if($p -and $p.Split(';') -contains $d){{ continue }}; if($p){{ $p = "$p;$d" }} else {{ $p = $d }} }}; [Environment]::SetEnvironmentVariable('PATH',$p,'User')"#,
        dirs_quoted = dirs
            .iter()
            .map(|d| format!("'{}'", d.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(","),
    );
    let ok = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map(|o| {
            if !o.status.success() {
                let msg = String::from_utf8_lossy(&o.stderr);
                emit_log(app, &format!("PowerShell 修改 PATH 失败：{}", msg.trim()));
            }
            o.status.success()
        })
        .unwrap_or(false);
    ok
}

#[cfg(target_os = "windows")]
fn try_symlink_from_which(app: &AppHandle, _which_cmd: &str, _home_dir: &std::path::Path) {
    let verified = run_in_shell("openclaw --version")
        .map(|o| o.status.success())
        .unwrap_or(false);
    if verified {
        emit_log(app, "openclaw 已在系统 PATH 中，命令行可直接使用。");
        return;
    }

    // 尝试找到 openclaw 所在目录并自动加入用户 PATH
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let fnm_node_dir = std::path::PathBuf::from(&appdata).join("fnm").join("node-versions");
    let mut dirs_to_add: Vec<String> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&fnm_node_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("v22.") {
                let install_dir = entry.path().join("installation");
                if install_dir.exists() {
                    dirs_to_add.push(install_dir.to_string_lossy().to_string());
                }
            }
        }
    }
    let npm_global = std::path::PathBuf::from(&appdata).join("npm");
    if npm_global.exists() {
        dirs_to_add.push(npm_global.to_string_lossy().to_string());
    }

    if !dirs_to_add.is_empty() {
        let added = try_add_to_user_path_windows(app, &dirs_to_add);
        if added {
            emit_log(app, "已自动将 Node.js 和 openclaw 路径添加到用户 PATH（重启终端后生效）。");
        } else {
            emit_log(app, "⚠ 自动修改 PATH 失败，请手动将以下目录添加到系统 PATH 环境变量：");
            for d in &dirs_to_add {
                emit_log(app, &format!("  {}", d));
            }
        }
    }

    emit_log(app, "");
    emit_log(app, "提示：如需在终端中使用 fnm 管理 Node 版本，请在 PowerShell 配置文件中添加：");
    emit_log(app, "  fnm env --use-on-cd | Out-String | Invoke-Expression");
}

#[cfg(target_os = "windows")]
fn try_symlink_bundled_fnm(app: &AppHandle, fnm_dir: &str, _home_dir: &std::path::Path) {
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
        .map(|e| e.path().join("installation"));

    match bin_dir {
        Some(bin) => {
            let dir_str = bin.to_string_lossy().to_string();
            emit_log(app, &format!("内置 fnm Node.js 位于 {}", dir_str));
            let added = try_add_to_user_path_windows(app, &[dir_str]);
            if added {
                emit_log(app, "已自动将上述目录添加到用户 PATH（重启终端后生效）。");
            } else {
                emit_log(app, "如需在命令行中直接使用 openclaw，请将上述目录添加到系统 PATH 环境变量。");
            }
        }
        None => {
            emit_log(app, "⚠ 未在内置 fnm 目录中找到 Node.js，请确认安装成功。");
        }
    }
}

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
    #[cfg(not(target_os = "windows"))]
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

    // 版本管理器安装失败时自动回退到 BundledFnm
    let mut use_bundled_fnm = false;

    match &strategy {
        NodeStrategy::SystemNode(v) => {
            emit_log(app, &format!("系统 Node.js v{} 满足要求（>= 22），跳过安装。", v));
            emit_step(app, step1, "done");
        }
        NodeStrategy::SystemFnm => {
            emit_log(app, "正在通过系统 fnm 安装 Node.js 22 并设为默认版本...");
            let fnm_cmd = format!(
                "fnm --node-dist-mirror {} install 22 && fnm default 22",
                NODE_DIST_MIRROR
            );
            match run_login_shell_step(app, &fnm_cmd, cancel_rx).await {
                Ok(()) => emit_step(app, step1, "done"),
                Err(e) => {
                    emit_log(app, &format!("⚠ 系统 fnm 安装失败（{}），自动切换到内置 fnm...", e));
                    use_bundled_fnm = true;
                }
            }
        }
        NodeStrategy::SystemNvm => {
            emit_log(app, "正在通过系统 nvm 安装 Node.js 22 并设为默认版本...");
            #[cfg(not(target_os = "windows"))]
            let cmd = format!("source '{}' && nvm install 22 && nvm alias default 22", nvm_sh.display());
            #[cfg(target_os = "windows")]
            let cmd = "nvm install 22 && nvm use 22".to_string();
            match run_login_shell_step(app, &cmd, cancel_rx).await {
                Ok(()) => emit_step(app, step1, "done"),
                Err(e) => {
                    emit_log(app, &format!("⚠ 系统 nvm 安装失败（{}），自动切换到内置 fnm...", e));
                    use_bundled_fnm = true;
                }
            }
        }
        NodeStrategy::BundledFnm => {
            use_bundled_fnm = true;
        }
    }

    if use_bundled_fnm {
        emit_log(app, "正在通过内置 fnm 安装 Node.js 22（首次下载约需 1~2 分钟）...");
        emit_log(app, &format!("内置 fnm 目录：{}", fnm_dir));
        match run_step(app, fnm_dir, &["install", "22"], cancel_rx).await {
            Ok(()) => emit_step(app, step1, "done"),
            Err(e) => { emit_step(app, step1, "error"); emit_error(app, step1, &e); return Err(e); }
        }
    }

    // ── 步骤 1.5（Windows）：确保 Git 可用 ─────────────────────────────────
    #[cfg(target_os = "windows")]
    {
        if !ensure_git_available(app) {
            let step_git = "ensure-git";
            emit_step(app, step_git, "error");
            emit_error(app, step_git, "缺少 Git，无法继续安装 openclaw");
            return Err("缺少 Git，请安装 Git for Windows 后重试".to_string());
        }
    }

    // ── 步骤 2：npm install -g openclaw ──────────────────────────────────────
    let step2 = "install-openclaw";
    emit_step(app, step2, "running");
    emit_log(app, "正在通过 npm 全局安装 openclaw，请稍候...");

    #[cfg(target_os = "windows")]
    let npm_bin = "npm.cmd";
    #[cfg(not(target_os = "windows"))]
    let npm_bin = "npm";

    let install_result = if use_bundled_fnm {
        run_step(app, fnm_dir, &["exec", "--using=22", "--", npm_bin, "install", "-g", "openclaw", "--no-update-notifier", "--no-fund"], cancel_rx).await
    } else {
        match &strategy {
            NodeStrategy::SystemNode(_) =>
                run_login_shell_step(app, "npm install -g openclaw --no-update-notifier --no-fund", cancel_rx).await,
            NodeStrategy::SystemFnm => {
                let cmd = format!("fnm exec --using=22 -- {} install -g openclaw --no-update-notifier --no-fund", npm_bin);
                run_login_shell_step(app, &cmd, cancel_rx).await
            }
            NodeStrategy::SystemNvm => {
                #[cfg(not(target_os = "windows"))]
                let cmd = format!("source '{}' && nvm exec 22 npm install -g openclaw --no-update-notifier --no-fund", nvm_sh.display());
                #[cfg(target_os = "windows")]
                let cmd = "npm install -g openclaw --no-update-notifier --no-fund".to_string();
                run_login_shell_step(app, &cmd, cancel_rx).await
            }
            NodeStrategy::BundledFnm => unreachable!(),
        }
    };

    match install_result {
        Ok(()) => emit_step(app, step2, "done"),
        Err(e) => {
            emit_step(app, step2, "error");
            emit_log(app, "");
            emit_log(app, "npm 安装失败，常见原因及解决方法：");
            emit_log(app, "  1. 缺少 Git：某些 npm 依赖需要 git。");
            emit_log(app, "     Windows 请安装 Git for Windows：https://git-scm.com/download/win");
            emit_log(app, "  2. 网络问题（大陆用户）：可尝试切换 npm 镜像源后重试");
            emit_log(app, "     npm config set registry https://registry.npmmirror.com");
            emit_log(app, "  3. 权限问题：尝试在终端手动执行 npm install -g openclaw");
            emit_log(app, "  4. 详细日志见 ~/.npm/_logs/ 目录下最新的 debug 文件");
            emit_error(app, step2, &e);
            return Err(e);
        }
    }

    // ── 安装后处理：确保 openclaw 在终端 PATH 中 ─────────────────────────────
    if use_bundled_fnm {
        try_symlink_bundled_fnm(app, fnm_dir, &home_dir);
    } else {
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
            NodeStrategy::BundledFnm => unreachable!(),
        }
    }

    write_installed_marker(app);

    // ── 最终验证：在登录 shell 中确认 openclaw 可访问 ──────────────────────
    let verified = run_in_shell("openclaw --version")
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
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string());
            #[cfg(target_os = "windows")]
            { format!("{}\\AppData\\Local\\claw-browser\\fnm", home) }
            #[cfg(not(target_os = "windows"))]
            { format!("{}/.local/share/claw-browser/fnm", home) }
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

    let flag_path = app.path().app_data_dir()
        .ok()
        .map(|d| d.join("openclaw-npm-installed.flag"));

    let flag_exists = flag_path.as_ref()
        .map(|p| p.exists())
        .unwrap_or(false);

    // flag 存在时，验证 openclaw 命令是否真实可执行，
    // 避免卸载后残留 flag 文件导致误判为已安装。
    let npm_installed = if onboarded {
        true
    } else if flag_exists {
        let actually_installed = run_in_shell("openclaw --version")
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !actually_installed {
            // 清理残留 flag，下次直接走安装流程
            if let Some(p) = &flag_path {
                let _ = std::fs::remove_file(p);
            }
        }
        actually_installed
    } else {
        false
    };

    OpenclawInstallStatus { npm_installed, onboarded }
}

#[tauri::command]
pub async fn cancel_install(app: AppHandle) -> Result<(), String> {
    let state = app.try_state::<InstallerState>().ok_or("InstallerState not found")?;
    let mut guard = state.cancel_tx.lock().unwrap();
    if let Some(tx) = guard.take() { let _ = tx.send(()); }
    Ok(())
}

