//! OpenClaw 配置（onboard）流程
//!
//! 三种模式：
//! - 非交互式：`run_onboard` 通过 CLI 参数调用 `openclaw onboard --non-interactive`
//! - 交互式 PTY（终端）：`start_onboard_pty` 在内嵌终端中运行真实 TUI（仅 Unix）
//! - 卡片向导：`start_onboard_wizard` 用跨平台 PTY 驱动真实 TUI，
//!   解析屏幕输出为结构化 prompt 事件，前端渲染为卡片，用户点击按钮交互

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

use crate::installer::detect_login_shell;

// ─── Onboard（非交互式）────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct OnboardParams {
    pub auth_choice: String,
    pub api_key: String,
    #[serde(default)]
    pub custom_base_url: String,
    #[serde(default)]
    pub custom_model_id: String,
}

#[derive(Clone, serde::Serialize)]
struct OnboardLogPayload { line: String }

#[derive(Clone, serde::Serialize)]
struct OnboardDonePayload { success: bool, error: Option<String> }

#[tauri::command]
pub async fn run_onboard(app: AppHandle, params: OnboardParams) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let shell = detect_login_shell();

    let mut args = vec![
        "openclaw".to_string(),
        "onboard".to_string(),
        "--non-interactive".to_string(),
        "--auth-choice".to_string(),
        params.auth_choice.clone(),
        "--secret-input-mode".to_string(),
        "plaintext".to_string(),
    ];

    match params.auth_choice.as_str() {
        "anthropic-api-key" => {
            args.push("--anthropic-api-key".to_string());
            args.push(params.api_key.clone());
        }
        "openai-api-key" => {
            args.push("--openai-api-key".to_string());
            args.push(params.api_key.clone());
        }
        "custom-api-key" => {
            args.push("--custom-api-key".to_string());
            args.push(params.api_key.clone());
            if !params.custom_base_url.is_empty() {
                args.push("--custom-base-url".to_string());
                args.push(params.custom_base_url.clone());
            }
            if !params.custom_model_id.is_empty() {
                args.push("--custom-model-id".to_string());
                args.push(params.custom_model_id.clone());
            }
            args.push("--custom-compatibility".to_string());
            args.push("openai".to_string());
        }
        _ => {}
    }

    let cmd_str = args.iter()
        .map(|a| if a.contains(' ') { format!("'{}'", a) } else { a.clone() })
        .collect::<Vec<_>>()
        .join(" ");

    let _ = app.emit("onboard:log", OnboardLogPayload { line: format!("> {}", cmd_str) });

    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new(&shell)
        .args(["-l", "-c", &cmd_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 onboard 失败：{}", e))?;

    #[cfg(target_os = "windows")]
    let mut child = Command::new("cmd")
        .args(["/C", &cmd_str])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 onboard 失败：{}", e))?;

    let mut out = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut err = BufReader::new(child.stderr.take().unwrap()).lines();
    let (mut out_done, mut err_done) = (false, false);

    loop {
        if out_done && err_done { break; }
        tokio::select! {
            line = out.next_line(), if !out_done => match line {
                Ok(Some(l)) => { let _ = app.emit("onboard:log", OnboardLogPayload { line: l }); }
                _ => out_done = true,
            },
            line = err.next_line(), if !err_done => match line {
                Ok(Some(l)) => { let _ = app.emit("onboard:log", OnboardLogPayload { line: l }); }
                _ => err_done = true,
            },
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if status.success() {
        let _ = app.emit("onboard:done", OnboardDonePayload { success: true, error: None });
        Ok(())
    } else {
        let msg = format!("onboard 退出码 {}", status.code().unwrap_or(-1));
        let _ = app.emit("onboard:done", OnboardDonePayload { success: false, error: Some(msg.clone()) });
        Err(msg)
    }
}

// ─── 卡片向导：跨平台 PTY + 屏幕解析 → 结构化 prompt 事件 ─────────────────

/// 从 vt100 虚拟终端屏幕内容中解析出当前 prompt 类型和内容。
/// clack/prompts TUI 使用特定 Unicode 字符标记：
///   ◆ = 活跃 prompt   ◇ = 已完成   ● = 选中   ○ = 未选中
///   ┌ = intro          └ = outro     │ = 内容行
#[derive(Clone, serde::Serialize)]
pub struct WizardPrompt {
    /// "confirm" | "select" | "multiselect" | "input" | "password" | "info" | "done"
    pub prompt_type: String,
    pub question: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub selected: usize,
    /// 多选模式下的选中索引列表
    #[serde(default)]
    pub checked: Vec<usize>,
    /// 校验错误提示（如 "Please select at least one option"）
    #[serde(default)]
    pub error: Option<String>,
}

/// cursor_row: 终端光标所在行号（0-based），由 vt100::Screen::cursor_position() 提供。
/// 用于精确判定 multiselect / select 中哪个选项处于高亮（光标）状态。
fn parse_screen_for_prompt(screen_text: &str, cursor_row: u16) -> Option<WizardPrompt> {
    let lines: Vec<&str> = screen_text.lines().collect();

    // 检查是否已结束（outro）
    if lines.iter().any(|l| {
        let t = l.trim();
        t.starts_with('└')
    }) {
        let outro_text = lines.iter()
            .filter(|l| l.trim().starts_with('└'))
            .map(|l| l.trim().trim_start_matches('└').trim().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let has_active = lines.iter().any(|l| l.trim().starts_with('◆'));
        if !has_active {
            return Some(WizardPrompt {
                prompt_type: "done".to_string(),
                question: if outro_text.is_empty() { "完成".to_string() } else { outro_text },
                options: vec![],
                selected: 0,
                checked: vec![],
                error: None,
            });
        }
    }

    // 找最后一个活跃 prompt（◆）
    let prompt_idx = lines.iter().rposition(|l| l.trim().starts_with('◆'))?;
    let question = lines[prompt_idx]
        .trim()
        .trim_start_matches('◆')
        .trim()
        .to_string();

    // 收集 prompt 后面的 │ 行，同时记录每行在 screen 中的原始行号
    // (screen_row, content_text)
    let mut body_entries: Vec<(usize, String)> = Vec::new();
    let mut error_msg: Option<String> = None;

    for i in (prompt_idx + 1)..lines.len() {
        let t = lines[i].trim();
        if t.starts_with('│') {
            let content = t.trim_start_matches('│').trim();
            if content.to_lowercase().contains("please select") || content.to_lowercase().contains("required") {
                error_msg = Some(content.to_string());
            } else {
                body_entries.push((i, content.to_string()));
            }
        } else if t.starts_with('└') || t.starts_with('◆') || t.starts_with('◇') {
            break;
        }
    }

    let body_lines: Vec<String> = body_entries.iter().map(|(_, c)| c.clone()).collect();

    // 判断 multiselect（每行有 ◻ 或 ◼ 或 ☑ 或 ☐）
    // 保留 screen_row 用于光标位置映射
    let multi_entries: Vec<(usize, bool, String)> = body_entries.iter()
        .filter(|(_, c)| c.contains('◻') || c.contains('◼') || c.contains('☐') || c.contains('☑'))
        .map(|(row, c)| {
            let is_checked = c.contains('◼') || c.contains('☑');
            let text = c.replace('◻', "").replace('◼', "").replace('☐', "").replace('☑', "").trim().to_string();
            (*row, is_checked, text)
        })
        .filter(|(_, _, t)| !t.is_empty())
        .collect();

    if multi_entries.len() >= 2 {
        let mut checked = Vec::new();
        let mut options = Vec::new();
        let mut selected = 0;
        let cr = cursor_row as usize;

        for (opt_idx, (screen_row, is_checked, text)) in multi_entries.iter().enumerate() {
            if *is_checked { checked.push(opt_idx); }
            if *screen_row == cr { selected = opt_idx; }
            options.push(text.clone());
        }

        // 如果光标行没精确命中任何选项行（可能差一行），找最近的
        if selected == 0 && cr > 0 {
            let mut min_dist = usize::MAX;
            for (opt_idx, (screen_row, _, _)) in multi_entries.iter().enumerate() {
                let dist = (*screen_row as isize - cr as isize).unsigned_abs();
                if dist < min_dist {
                    min_dist = dist;
                    selected = opt_idx;
                }
            }
        }

        return Some(WizardPrompt {
            prompt_type: "multiselect".to_string(),
            question,
            options,
            selected,
            checked,
            error: error_msg,
        });
    }

    // 判断 confirm（单行 "○ Yes / ● No" 或 "● Yes / ○ No"）
    if body_lines.len() == 1 {
        let line = &body_lines[0];
        if (line.contains("Yes") || line.contains("yes"))
            && (line.contains("No") || line.contains("no"))
            && (line.contains('○') || line.contains('●'))
        {
            let selected = if line.contains("● Yes") || line.contains("● yes") { 0 } else { 1 };
            return Some(WizardPrompt {
                prompt_type: "confirm".to_string(),
                question,
                options: vec!["Yes".to_string(), "No".to_string()],
                selected,
                checked: vec![],
                error: error_msg,
            });
        }
    }

    // 判断 select（多行，每行有 ○ 或 ●）
    let radio_entries: Vec<(usize, bool, String)> = body_entries.iter()
        .filter(|(_, c)| c.contains('●') || c.contains('○'))
        .map(|(row, c)| {
            let is_sel = c.contains('●');
            let text = c.replace('●', "").replace('○', "").trim().to_string();
            (*row, is_sel, text)
        })
        .filter(|(_, _, t)| !t.is_empty())
        .collect();

    if radio_entries.len() >= 2 {
        let cr = cursor_row as usize;
        let mut selected = 0;
        let options: Vec<String> = radio_entries.iter().enumerate().map(|(i, (row, is_sel, text))| {
            // 优先用 cursor_row，兜底用 ● 标记
            if *row == cr { selected = i; }
            else if *is_sel && selected == 0 { selected = i; }
            text.clone()
        }).collect();
        return Some(WizardPrompt {
            prompt_type: "select".to_string(),
            question,
            options,
            selected,
            checked: vec![],
            error: error_msg,
        });
    }

    // 判断 password（body 含有 ▪ 遮罩字符）
    if body_lines.iter().any(|l| l.contains('▪')) {
        return Some(WizardPrompt {
            prompt_type: "password".to_string(),
            question,
            options: vec![],
            selected: 0,
            checked: vec![],
            error: error_msg,
        });
    }

    // 兜底：text input（body 为空或只有光标区域）
    Some(WizardPrompt {
        prompt_type: "input".to_string(),
        question,
        options: vec![],
        selected: 0,
        checked: vec![],
        error: error_msg,
    })
}

/// 向导状态：持有 PTY writer 和子进程
pub struct OnboardWizardState {
    inner: Arc<Mutex<Option<WizardInner>>>,
}

struct WizardInner {
    writer: Box<dyn std::io::Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl Default for OnboardWizardState {
    fn default() -> Self {
        Self { inner: Arc::new(Mutex::new(None)) }
    }
}

/// 返回补充 PATH 用的目录列表（跨平台版本）。
fn wizard_extra_path_dirs(app: &AppHandle) -> Vec<String> {
    let home = app.path().home_dir().ok()
        .and_then(|p| p.to_str().map(String::from))
        .or_else(|| std::env::var("HOME").ok())
        .or_else(|| std::env::var("USERPROFILE").ok())
        .unwrap_or_default();
    let mut dirs: Vec<String> = Vec::new();

    #[cfg(not(target_os = "windows"))]
    {
        dirs.push(format!("{}/.local/bin", home));
        #[cfg(target_os = "macos")]
        dirs.push("/opt/homebrew/bin".to_string());
        dirs.push("/usr/local/bin".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            dirs.push(format!("{}\\npm", appdata));
            dirs.push(format!("{}\\nvm", appdata));
        }
    }

    let openclaw_bin = if cfg!(windows) { "openclaw.cmd" } else { "openclaw" };

    // 扫描 nvm 所有版本
    #[cfg(not(target_os = "windows"))]
    let nvm_versions = std::path::Path::new(&home).join(".nvm").join("versions").join("node");
    #[cfg(target_os = "windows")]
    let nvm_versions = std::env::var("APPDATA")
        .map(|a| std::path::PathBuf::from(a).join("nvm"))
        .unwrap_or_else(|_| std::path::PathBuf::from(&home).join("AppData").join("Roaming").join("nvm"));
    if let Ok(rd) = std::fs::read_dir(&nvm_versions) {
        for e in rd.flatten() {
            let bin = if cfg!(windows) { e.path() } else { e.path().join("bin") };
            if bin.join(openclaw_bin).exists() {
                dirs.push(bin.to_string_lossy().to_string());
            }
        }
    }

    // 扫描系统 fnm
    #[cfg(not(target_os = "windows"))]
    let fnm_base = std::path::Path::new(&home).join(".local").join("share").join("fnm").join("node-versions");
    #[cfg(target_os = "windows")]
    let fnm_base = std::env::var("LOCALAPPDATA")
        .map(|a| std::path::PathBuf::from(a).join("fnm_multishells"))
        .unwrap_or_else(|_| std::path::PathBuf::from(&home).join("AppData").join("Local").join("fnm_multishells"));
    if let Ok(rd) = std::fs::read_dir(&fnm_base) {
        for e in rd.flatten() {
            let bin = if cfg!(windows) { e.path() } else { e.path().join("installation").join("bin") };
            if bin.join(openclaw_bin).exists() {
                dirs.push(bin.to_string_lossy().to_string());
            }
        }
    }

    // 扫描内置 fnm
    if let Ok(app_data) = app.path().app_data_dir() {
        let bundled = app_data.join("fnm").join("node-versions");
        if let Ok(rd) = std::fs::read_dir(&bundled) {
            for e in rd.flatten() {
                let bin = if cfg!(windows) {
                    e.path().join("installation")
                } else {
                    e.path().join("installation").join("bin")
                };
                if bin.join(openclaw_bin).exists() {
                    dirs.push(bin.to_string_lossy().to_string());
                }
            }
        }
    }
    dirs
}

/// 启动卡片向导：用 portable-pty 跨平台 PTY 运行 openclaw onboard，
/// 通过 vt100 解析屏幕，将 prompt 以结构化事件推送给前端。
#[tauri::command]
pub fn start_onboard_wizard(app: AppHandle) -> Result<(), String> {
    use portable_pty::{CommandBuilder, PtySize};
    use std::io::Read;

    let state = app.try_state::<OnboardWizardState>().ok_or("OnboardWizardState not found")?;
    {
        let g = state.inner.lock().unwrap();
        if g.is_some() {
            return Err("向导已在运行".to_string());
        }
    }

    let extra = wizard_extra_path_dirs(&app);
    let extra_str = extra.join(if cfg!(windows) { ";" } else { ":" });
    let current_path = std::env::var("PATH").unwrap_or_default();
    let full_path = format!("{}{}{}", extra_str, if cfg!(windows) { ";" } else { ":" }, current_path);

    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 30,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| format!("PTY 分配失败：{}", e))?;

    let mut cmd = if cfg!(windows) {
        let mut c = CommandBuilder::new("cmd");
        c.args(["/C", "openclaw onboard"]);
        c
    } else {
        let shell = detect_login_shell();
        let cmd_str = format!("export PATH=\"{}:$PATH\"; openclaw onboard", extra_str);
        let mut c = CommandBuilder::new(&shell);
        c.args(["-l", "-i", "-c", &cmd_str]);
        c
    };
    cmd.env("PATH", &full_path);

    let child = pair.slave.spawn_command(cmd)
        .map_err(|e| format!("启动 openclaw onboard 失败：{}", e))?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()
        .map_err(|e| format!("无法获取 PTY reader：{}", e))?;
    let writer = pair.master.take_writer()
        .map_err(|e| format!("无法获取 PTY writer：{}", e))?;

    let state_inner = state.inner.clone();
    {
        let mut g = state_inner.lock().unwrap();
        *g = Some(WizardInner { writer, child });
    }

    let app_emit = app.clone();
    let state_for_thread = state_inner.clone();

    // 读取线程：通过 channel 将 PTY 数据传出（避免阻塞 read 卡住解析逻辑）
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => { let _ = tx.send(buf[..n].to_vec()); }
                Err(_) => break,
            }
        }
    });

    // 解析线程：从 channel 收数据，debounce 后解析屏幕
    std::thread::spawn(move || {
        let mut parser = vt100::Parser::new(30, 120, 0);
        let mut last_prompt: Option<String> = None;
        let debounce = std::time::Duration::from_millis(150);

        loop {
            // 阻塞等待第一块数据
            match rx.recv() {
                Ok(data) => parser.process(&data),
                Err(_) => break, // 发送端关闭，进程已退出
            }
            // 持续消耗后续数据直到 debounce 超时（TUI 渲染完毕）
            loop {
                match rx.recv_timeout(debounce) {
                    Ok(data) => parser.process(&data),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            let screen = parser.screen();
            let screen_text = screen.contents();
            let cursor_row = screen.cursor_position().0;

            #[derive(Clone, serde::Serialize)]
            struct WizardScreen { text: String, cursor_row: u16 }
            let _ = app_emit.emit("wizard:screen", WizardScreen {
                text: screen_text.clone(),
                cursor_row,
            });

            if let Some(prompt) = parse_screen_for_prompt(&screen_text, cursor_row) {
                let prompt_key = format!(
                    "{}:{}:{:?}:{}:{:?}:{:?}",
                    prompt.prompt_type, prompt.question, prompt.options,
                    prompt.selected, prompt.checked, prompt.error,
                );
                if last_prompt.as_deref() != Some(&prompt_key) {
                    last_prompt = Some(prompt_key);
                    let _ = app_emit.emit("wizard:prompt", prompt);
                }
            }
        }

        // 进程结束，获取退出码
        let code = {
            let mut g = state_for_thread.lock().unwrap();
            let code = if let Some(ref mut inner) = *g {
                inner.child.wait()
                    .map(|s| s.exit_code() as i32)
                    .unwrap_or(-1)
            } else {
                -1
            };
            *g = None;
            code
        };

        #[derive(Clone, serde::Serialize)]
        struct WizardExited { code: i32 }
        let _ = app_emit.emit("wizard:exited", WizardExited { code });
    });

    Ok(())
}

fn action_to_bytes(action: &str) -> Result<Vec<u8>, String> {
    match action {
        "enter" => Ok(vec![b'\r']),
        "space" => Ok(vec![b' ']),
        "up" => Ok(vec![0x1b, b'[', b'A']),
        "down" => Ok(vec![0x1b, b'[', b'B']),
        "right" => Ok(vec![0x1b, b'[', b'C']),
        "left" => Ok(vec![0x1b, b'[', b'D']),
        "y" => Ok(vec![b'y']),
        "n" => Ok(vec![b'n']),
        _ if action.starts_with("text:") => Ok(action[5..].as_bytes().to_vec()),
        _ if action.starts_with("submit:") => {
            let mut d = action[7..].as_bytes().to_vec();
            d.push(b'\r');
            Ok(d)
        }
        _ => Err(format!("未知 action：{}", action)),
    }
}

/// 发送单个按键。
#[tauri::command]
pub fn wizard_send_key(state: tauri::State<'_, OnboardWizardState>, action: String) -> Result<(), String> {
    use std::io::Write;
    let mut g = state.inner.lock().unwrap();
    let inner = g.as_mut().ok_or("向导未在运行")?;
    let data = action_to_bytes(&action)?;
    inner.writer.write_all(&data).map_err(|e| e.to_string())?;
    inner.writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// 批量发送多个按键（一次加锁 + 一次 flush，减少延迟）。
#[tauri::command]
pub fn wizard_send_keys(state: tauri::State<'_, OnboardWizardState>, actions: Vec<String>) -> Result<(), String> {
    use std::io::Write;
    let mut g = state.inner.lock().unwrap();
    let inner = g.as_mut().ok_or("向导未在运行")?;
    for action in &actions {
        let data = action_to_bytes(action)?;
        inner.writer.write_all(&data).map_err(|e| e.to_string())?;
    }
    inner.writer.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// 终止向导进程。
#[tauri::command]
pub fn kill_onboard_wizard(state: tauri::State<'_, OnboardWizardState>) -> Result<(), String> {
    let mut g = state.inner.lock().unwrap();
    if let Some(mut inner) = g.take() {
        let _ = inner.child.kill();
    }
    Ok(())
}

/// 向导是否在运行。
#[tauri::command]
pub fn is_onboard_wizard_running(state: tauri::State<'_, OnboardWizardState>) -> bool {
    state.inner.lock().unwrap().is_some()
}

// ─── Onboard PTY（交互式 TUI，内嵌终端）──────────────────────────────────────

#[cfg(unix)]
#[derive(Clone, serde::Serialize)]
struct OnboardPtyOutputPayload {
    data: String,
}

#[cfg(unix)]
#[derive(Clone, serde::Serialize)]
struct OnboardPtyExitedPayload {
    code: i32,
}

/// 状态：stdin 写入通道 + 取消通道；writer 任务持有 write_pty。
#[cfg(unix)]
#[derive(Clone)]
pub struct OnboardPtyState {
    inner: Arc<Mutex<Option<(tokio::sync::mpsc::Sender<Vec<u8>>, tokio::sync::mpsc::Sender<()>)>>>,
}

#[cfg(unix)]
impl Default for OnboardPtyState {
    fn default() -> Self {
        Self { inner: Arc::new(Mutex::new(None)) }
    }
}

/// 在文件系统中扫描 nvm / fnm（系统+内置）所有 node 版本，
/// 找出含有 `openclaw` 可执行文件的 bin 目录并返回。
/// 不依赖 shell，不限定版本号，避免因用户在不同版本下全局安装而漏检。
#[cfg(unix)]
fn find_node_bin_dirs_with_openclaw(app: &AppHandle) -> Vec<String> {
    let home = app.path().home_dir().ok()
        .and_then(|p| p.to_str().map(String::from))
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_default();
    let mut found: Vec<String> = Vec::new();

    // ── nvm：~/.nvm/versions/node/v*/bin ──────────────────────────────────
    let nvm_versions = std::path::Path::new(&home).join(".nvm").join("versions").join("node");
    if let Ok(rd) = std::fs::read_dir(&nvm_versions) {
        for e in rd.flatten() {
            let bin = e.path().join("bin");
            if bin.join("openclaw").exists() {
                found.push(bin.to_string_lossy().to_string());
            }
        }
    }

    // ── 系统 fnm：~/.local/share/fnm/node-versions/v*/installation/bin ───
    let system_fnm = std::path::Path::new(&home).join(".local").join("share").join("fnm").join("node-versions");
    if let Ok(rd) = std::fs::read_dir(&system_fnm) {
        for e in rd.flatten() {
            let bin = e.path().join("installation").join("bin");
            if bin.join("openclaw").exists() {
                found.push(bin.to_string_lossy().to_string());
            }
        }
    }

    // ── 内置（BundledFnm）：app_data/fnm/node-versions/v*/installation/bin ─
    if let Ok(app_data) = app.path().app_data_dir() {
        let bundled = app_data.join("fnm").join("node-versions");
        if let Ok(rd) = std::fs::read_dir(&bundled) {
            for e in rd.flatten() {
                let bin = e.path().join("installation").join("bin");
                if bin.join("openclaw").exists() {
                    found.push(bin.to_string_lossy().to_string());
                }
            }
        }
    }

    found
}

/// 返回补充 PATH 用的目录列表：固定的全局 bin 目录 + 所有找到 openclaw 的 node bin 目录。
#[cfg(unix)]
fn extra_path_dirs(app: &AppHandle) -> Vec<String> {
    let home = app.path().home_dir().ok()
        .and_then(|p| p.to_str().map(String::from))
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_default();
    let mut dirs = vec![
        format!("{}/.local/bin", home),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
    ];
    dirs.extend(find_node_bin_dirs_with_openclaw(app));
    dirs
}

/// 启动交互式 openclaw onboard（PTY），输出与退出码通过事件推送。
///
/// 关键：使用 `-l -i -c` 让 zsh/bash 以「交互式登录 shell」运行，
/// 从而 source 所有 profile 文件（包括 .zshrc），nvm/fnm 自动初始化。
/// 仅用 `-l -c` 是非交互式，不会 source .zshrc，nvm/fnm 的 PATH 因此缺失。
#[cfg(unix)]
#[tauri::command]
pub async fn start_onboard_pty(app: AppHandle) -> Result<(), String> {
    use pty_process::{Command as PtyCommand, Size};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let shell = detect_login_shell();
    // 追加固定目录作为保底（BundledFnm 场景），不影响 nvm/fnm 用户
    let extra = extra_path_dirs(&app).join(":");
    let cmd_str = format!("export PATH=\"{}:$PATH\"; openclaw onboard", extra);

    let (pty, pts) = pty_process::open().map_err(|e| format!("PTY 分配失败：{}", e))?;
    pty.resize(Size::new(30, 120)).map_err(|e| format!("PTY resize 失败：{}", e))?;

    // -l: 登录 shell（source /etc/zprofile, ~/.zprofile）
    // -i: 交互式（source ~/.zshrc，nvm/fnm 在此初始化）
    // -c: 执行命令后退出
    let child = PtyCommand::new(&shell)
        .args(["-l", "-i", "-c", &cmd_str])
        .spawn(pts)
        .map_err(|e| format!("启动 openclaw onboard 失败：{}", e))?;

    let (read_pty, write_pty) = pty.into_split();
    let (cancel_tx, mut cancel_rx) = tokio::sync::mpsc::channel::<()>(1);
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    let state = app.try_state::<OnboardPtyState>().ok_or("OnboardPtyState not found")?;
    let state_inner = state.inner.clone();
    {
        let mut g = state.inner.lock().unwrap();
        *g = Some((stdin_tx, cancel_tx));
    }

    let app_emit = app.clone();
    tokio::spawn(async move {
        let mut read_pty = read_pty;
        let mut child = child;
        let mut buf = [0u8; 4096];
        loop {
            tokio::select! {
                n = read_pty.read(&mut buf) => {
                    match n {
                        Ok(0) => break,
                        Ok(n) => {
                            let s = String::from_utf8_lossy(&buf[..n]);
                            let _ = app_emit.emit("onboard:pty_output", OnboardPtyOutputPayload { data: s.to_string() });
                        }
                        Err(_) => break,
                    }
                }
                _ = cancel_rx.recv() => {
                    let _ = child.kill().await;
                    break;
                }
            }
        }
        let code = child.wait().await.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        let _ = app_emit.emit("onboard:pty_exited", OnboardPtyExitedPayload { code });
        let mut g = state_inner.lock().unwrap();
        *g = None;
    });

    tokio::spawn(async move {
        let mut write_pty = write_pty;
        while let Some(data) = stdin_rx.recv().await {
            let _ = write_pty.write_all(&data).await;
            let _ = write_pty.flush().await;
        }
    });

    Ok(())
}

/// 向正在运行的 onboard PTY 进程 stdin 写入数据（用户输入）。
#[cfg(unix)]
#[tauri::command]
pub fn write_onboard_stdin(state: tauri::State<'_, OnboardPtyState>, data: String) -> Result<(), String> {
    let g = state.inner.lock().unwrap();
    if let Some((ref stdin_tx, _)) = *g {
        stdin_tx.try_send(data.into_bytes()).map_err(|_| "发送失败（通道满或已关闭）".to_string())?;
        Ok(())
    } else {
        Err("onboard 未在运行".to_string())
    }
}

/// 结束正在运行的 onboard PTY 进程。
#[cfg(unix)]
#[tauri::command]
pub async fn kill_onboard_pty(state: tauri::State<'_, OnboardPtyState>) -> Result<(), String> {
    let mut g = state.inner.lock().unwrap();
    if let Some((_, ref cancel_tx)) = *g {
        let _ = cancel_tx.try_send(());
    }
    *g = None;
    Ok(())
}

/// 是否已有 onboard PTY 在运行
#[cfg(unix)]
#[tauri::command]
pub fn is_onboard_pty_running(state: tauri::State<'_, OnboardPtyState>) -> bool {
    state.inner.lock().unwrap().is_some()
}

// ─── Windows：无 PTY，提供空实现 ──────────────────────────────────────────

#[cfg(not(unix))]
pub struct OnboardPtyState;

#[cfg(not(unix))]
impl Default for OnboardPtyState {
    fn default() -> Self { Self }
}

#[cfg(not(unix))]
#[tauri::command]
pub async fn start_onboard_pty(_app: AppHandle) -> Result<(), String> {
    Err("当前系统不支持内嵌终端，请在系统终端执行 openclaw onboard".to_string())
}

#[cfg(not(unix))]
#[tauri::command]
pub async fn write_onboard_stdin(_state: tauri::State<'_, OnboardPtyState>, _data: String) -> Result<(), String> {
    Err("当前系统不支持".to_string())
}

#[cfg(not(unix))]
#[tauri::command]
pub async fn kill_onboard_pty(_state: tauri::State<'_, OnboardPtyState>) -> Result<(), String> {
    Ok(())
}

#[cfg(not(unix))]
#[tauri::command]
pub fn is_onboard_pty_running(_state: tauri::State<'_, OnboardPtyState>) -> bool {
    false
}
