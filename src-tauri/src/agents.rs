//! 智能体助手管理
//!
//! 驱动 `openclaw agents add <work>` 的跨平台 PTY 向导，
//! 复用 configure.rs 的屏幕解析逻辑。

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ─── 数据结构 ──────────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub workspace: String,
    pub description: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct WizardPrompt {
    pub prompt_type: String,
    pub question: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub selected: usize,
    #[serde(default)]
    pub checked: Vec<usize>,
    #[serde(default)]
    pub error: Option<String>,
    /// input/password 类型的当前编辑值（TUI 已填入的默认值）
    #[serde(default)]
    pub current_value: Option<String>,
}

// ─── 列出智能体 ────────────────────────────────────────────────────────────

/// 读取 ~/.openclaw/ 目录，扫描 workspace-<work> 目录，列出所有已配置的智能体。
/// 目录规律：work 名为 "work" 对应 workspace-work，默认 workspace 对应空 work 名。
#[tauri::command]
pub fn list_agents(app: AppHandle) -> Result<Vec<AgentInfo>, String> {
    let openclaw = crate::installer::openclaw_dir(&app)?;

    if !openclaw.exists() {
        return Ok(vec![]);
    }

    let mut agents = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&openclaw) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // 匹配 workspace 或 workspace-<work>
            let work = if dir_name == "workspace" {
                "".to_string()
            } else if let Some(w) = dir_name.strip_prefix("workspace-") {
                w.to_string()
            } else {
                continue;
            };

            // workspace 目录下须有 IDENTITY.md 才算有效智能体
            if !path.join("IDENTITY.md").exists() { continue; }

            let display_name = if work.is_empty() { "default".to_string() } else { work.clone() };
            agents.push(AgentInfo {
                name: display_name,
                workspace: work,
                description: None,
            });
        }
    }
    Ok(agents)
}

// ─── 智能体文件读写 ────────────────────────────────────────────────────────

const AGENT_FILES: &[&str] = &["IDENTITY.md", "USER.md", "SOUL.md"];

/// 返回智能体 workspace 目录：
/// - work 为空或 "default" → ~/.openclaw/workspace
/// - 否则                   → ~/.openclaw/workspace-<work>
fn agent_dir(app: &AppHandle, work: &str) -> Result<std::path::PathBuf, String> {
    let base = crate::installer::openclaw_dir(app)?;
    let dir_name = if work.is_empty() || work == "default" {
        "workspace".to_string()
    } else {
        format!("workspace-{}", work)
    };
    Ok(base.join(dir_name))
}

/// 列出智能体目录下存在的文件（固定三个，返回每个文件是否存在）
#[tauri::command]
pub fn list_agent_files(app: AppHandle, work: String) -> Result<Vec<String>, String> {
    let dir = agent_dir(&app, &work)?;
    Ok(AGENT_FILES.iter()
        .filter(|f| dir.join(f).exists())
        .map(|f| f.to_string())
        .collect())
}

/// 读取智能体文件内容；文件不存在时返回空字符串
#[tauri::command]
pub fn read_agent_file(app: AppHandle, work: String, filename: String) -> Result<String, String> {
    if !AGENT_FILES.contains(&filename.as_str()) {
        return Err(format!("不允许的文件名：{}", filename));
    }
    let path = agent_dir(&app, &work)?.join(&filename);
    if !path.exists() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// 写入智能体文件内容；目录不存在时自动创建
#[tauri::command]
pub fn write_agent_file(
    app: AppHandle,
    work: String,
    filename: String,
    content: String,
) -> Result<(), String> {
    if !AGENT_FILES.contains(&filename.as_str()) {
        return Err(format!("不允许的文件名：{}", filename));
    }
    let dir = agent_dir(&app, &work)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(&filename), content).map_err(|e| e.to_string())
}

// ─── 向导状态 ──────────────────────────────────────────────────────────────

pub struct AgentWizardState {
    inner: Arc<Mutex<Option<WizardInner>>>,
}

struct WizardInner {
    writer: Box<dyn std::io::Write + Send>,
    #[cfg(not(target_os = "windows"))]
    child: Box<dyn portable_pty::Child + Send + Sync>,
    #[cfg(target_os = "windows")]
    proc: conpty::Process,
}

impl WizardInner {
    fn kill(&mut self) {
        #[cfg(not(target_os = "windows"))]
        { let _ = self.child.kill(); }
        #[cfg(target_os = "windows")]
        { let _ = self.proc.exit(1); }
    }

    fn wait_exit_code(&mut self) -> i32 {
        #[cfg(not(target_os = "windows"))]
        { self.child.wait().map(|s| s.exit_code() as i32).unwrap_or(-1) }
        #[cfg(target_os = "windows")]
        { self.proc.wait(None).map(|code| code as i32).unwrap_or(-1) }
    }
}

impl Default for AgentWizardState {
    fn default() -> Self {
        Self { inner: Arc::new(Mutex::new(None)) }
    }
}

// ─── 屏幕解析（复用 configure.rs 逻辑）────────────────────────────────────

/// 去除 TUI 光标块字符（█ U+2588，及其他常见块状光标），得到干净的输入值。
fn strip_cursor_chars(s: &str) -> String {
    s.chars()
        .filter(|&c| c != '\u{2588}' && c != '\u{2589}' && c != '\u{258A}' && c != '\u{258B}'
                  && c != '\u{2502}' )
        .collect::<String>()
        .trim()
        .to_string()
}

fn parse_screen_for_prompt(screen_text: &str, cursor_row: u16) -> Option<WizardPrompt> {
    let lines: Vec<&str> = screen_text.lines().collect();

    // 检查完成（outro └）
    if lines.iter().any(|l| l.trim().starts_with('└')) {
        let has_active = lines.iter().any(|l| l.trim().starts_with('◆'));
        let has_option_markers = lines.iter().any(|l| {
            let t = l.trim();
            t.contains('○') || t.contains('●') || t.contains('◻') || t.contains('◼')
        });
        if !has_active && !has_option_markers {
            let outro_text: String = lines.iter()
                .filter(|l| l.trim().starts_with('└'))
                .map(|l| l.trim().trim_start_matches('└').trim().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            return Some(WizardPrompt {
                prompt_type: "done".to_string(),
                question: if outro_text.is_empty() { "完成".to_string() } else { outro_text },
                options: vec![],
                selected: 0,
                checked: vec![],
                error: None,
                current_value: None,
            });
        }
    }

    let prompt_idx = lines.iter().rposition(|l| l.trim().starts_with('◆'));
    let question = match prompt_idx {
        Some(idx) => lines[idx].trim().trim_start_matches('◆').trim().to_string(),
        None => String::new(),
    };
    let scan_start = match prompt_idx {
        Some(idx) => idx + 1,
        None => 0,
    };

    let mut body_entries: Vec<(usize, String)> = Vec::new();
    let mut error_msg: Option<String> = None;

    for i in scan_start..lines.len() {
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

    // multiselect
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
        if selected == 0 && cr > 0 {
            let mut min_dist = usize::MAX;
            for (opt_idx, (screen_row, _, _)) in multi_entries.iter().enumerate() {
                let dist = (*screen_row as isize - cr as isize).unsigned_abs();
                if dist < min_dist { min_dist = dist; selected = opt_idx; }
            }
        }
        return Some(WizardPrompt {
            prompt_type: "multiselect".to_string(),
            question, options, selected, checked, error: error_msg,
            current_value: None,
        });
    }

    // confirm
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
                current_value: None,
            });
        }
    }

    // select
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
            if *row == cr { selected = i; }
            else if *is_sel && selected == 0 { selected = i; }
            text.clone()
        }).collect();
        return Some(WizardPrompt {
            prompt_type: "select".to_string(),
            question, options, selected, checked: vec![], error: error_msg,
            current_value: None,
        });
    }

    // password（▪ 遮罩，不提取真实值）
    if body_lines.iter().any(|l| l.contains('▪')) {
        return Some(WizardPrompt {
            prompt_type: "password".to_string(),
            question, options: vec![], selected: 0, checked: vec![], error: error_msg,
            current_value: None,
        });
    }

    // input：从 body 第一行提取当前编辑值（去掉光标块字符）
    let current_value = body_lines.first()
        .map(|s| strip_cursor_chars(s))
        .filter(|s| !s.is_empty());

    Some(WizardPrompt {
        prompt_type: "input".to_string(),
        question, options: vec![], selected: 0, checked: vec![], error: error_msg,
        current_value,
    })
}

// ─── 启动向导 ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn start_agent_add_wizard(app: AppHandle, work: String) -> Result<(), String> {
    use std::io::Read;
    use crate::configure::wizard_extra_path_dirs;

    let state = app.try_state::<AgentWizardState>().ok_or("AgentWizardState not found")?;
    {
        let g = state.inner.lock().unwrap();
        if g.is_some() { return Err("向导已在运行".to_string()); }
    }

    let extra = wizard_extra_path_dirs(&app);
    let extra_str = extra.join(if cfg!(windows) { ";" } else { ":" });
    let current_path = std::env::var("PATH").unwrap_or_default();
    let full_path = if extra_str.is_empty() {
        current_path.clone()
    } else {
        format!("{}{}{}", extra_str, if cfg!(windows) { ";" } else { ":" }, current_path)
    };

    // ── macOS/Linux ──────────────────────────────────────────────────────
    #[cfg(not(target_os = "windows"))]
    let (reader, inner): (Box<dyn Read + Send>, WizardInner) = {
        use portable_pty::{CommandBuilder, PtySize};
        use crate::installer::detect_login_shell;

        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 30, cols: 120, pixel_width: 0, pixel_height: 0,
        }).map_err(|e| format!("PTY 分配失败：{}", e))?;

        let shell = detect_login_shell();
        let cmd_str = format!(
            "export PATH=\"{}:$PATH\"; openclaw agents add {}",
            extra_str,
            shell_escape(&work)
        );
        let mut cmd = CommandBuilder::new(&shell);
        cmd.args(["-l", "-i", "-c", &cmd_str]);
        cmd.env("PATH", &full_path);

        let child = pair.slave.spawn_command(cmd)
            .map_err(|e| format!("启动 openclaw agents add 失败：{}", e))?;
        drop(pair.slave);

        let reader: Box<dyn Read + Send> = pair.master.try_clone_reader()
            .map_err(|e| format!("无法获取 PTY reader：{}", e))?;
        let writer = pair.master.take_writer()
            .map_err(|e| format!("无法获取 PTY writer：{}", e))?;

        (reader, WizardInner { writer, child })
    };

    // ── Windows ──────────────────────────────────────────────────────────
    #[cfg(target_os = "windows")]
    let (reader, inner): (Box<dyn Read + Send>, WizardInner) = {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", "openclaw", "agents", "add", &work]);
        cmd.env("TERM", "xterm-256color");
        cmd.env("FORCE_COLOR", "1");
        cmd.env("PATH", &full_path);

        let mut opts = conpty::ProcessOptions::default();
        opts.set_console_size(Some((120, 30)));
        let mut proc = opts.spawn(cmd)
            .map_err(|e| format!("启动 openclaw agents add 失败：{}", e))?;

        let reader: Box<dyn Read + Send> = Box::new(
            proc.output().map_err(|e| format!("无法获取 PTY reader：{}", e))?
        );
        let writer: Box<dyn std::io::Write + Send> = Box::new(
            proc.input().map_err(|e| format!("无法获取 PTY writer：{}", e))?
        );
        (reader, WizardInner { writer, proc })
    };

    // ── 公共：存储状态 + 启动读取/解析线程 ──────────────────────────────
    let state_inner = state.inner.clone();
    { let mut g = state_inner.lock().unwrap(); *g = Some(inner); }

    let app_for_reader = app.clone();
    let app_emit = app.clone();
    let state_for_thread = state_inner.clone();

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = app_for_reader.emit("agent_wizard:raw-data",
                        format!("[reader] {} bytes", n));
                    let _ = tx.send(buf[..n].to_vec());
                }
                Err(_) => break,
            }
        }
    });

    std::thread::spawn(move || {
        let mut parser = vt100::Parser::new(30, 120, 0);
        let mut last_prompt: Option<String> = None;
        let debounce = std::time::Duration::from_millis(150);

        loop {
            match rx.recv() {
                Ok(data) => { parser.process(&data); }
                Err(_) => break,
            }
            loop {
                match rx.recv_timeout(debounce) {
                    Ok(data) => { parser.process(&data); }
                    Err(_) => break,
                }
            }
            let screen = parser.screen();
            let screen_text = screen.contents();
            let cursor_row = screen.cursor_position().0;

            #[derive(Clone, serde::Serialize)]
            struct WizardScreen { text: String, cursor_row: u16 }
            let _ = app_emit.emit("agent_wizard:screen", WizardScreen {
                text: screen_text.clone(), cursor_row,
            });

            if let Some(prompt) = parse_screen_for_prompt(&screen_text, cursor_row) {
                let key = format!(
                    "{}:{}:{:?}:{}:{:?}:{:?}",
                    prompt.prompt_type, prompt.question, prompt.options,
                    prompt.selected, prompt.checked, prompt.error,
                );
                if last_prompt.as_deref() != Some(&key) {
                    last_prompt = Some(key);
                    let _ = app_emit.emit("agent_wizard:prompt", prompt);
                }
            }
        }

        let code: i32 = {
            let mut g = state_for_thread.lock().unwrap();
            let code = if let Some(ref mut inner) = *g {
                inner.wait_exit_code()
            } else { -1 };
            *g = None;
            code
        };

        #[derive(Clone, serde::Serialize)]
        struct WizardExited { code: i32 }
        let _ = app_emit.emit("agent_wizard:exited", WizardExited { code });
    });

    Ok(())
}

fn shell_escape(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn action_to_bytes(action: &str) -> Result<Vec<u8>, String> {
    match action {
        "enter" => Ok(vec![b'\r']),
        "space" => Ok(vec![b' ']),
        "up"    => Ok(vec![0x1b, b'[', b'A']),
        "down"  => Ok(vec![0x1b, b'[', b'B']),
        "right" => Ok(vec![0x1b, b'[', b'C']),
        "left"  => Ok(vec![0x1b, b'[', b'D']),
        _ if action.starts_with("submit:") => {
            let mut d = action[7..].as_bytes().to_vec();
            d.push(b'\r');
            Ok(d)
        }
        _ if action.starts_with("text:") => Ok(action[5..].as_bytes().to_vec()),
        _ => Err(format!("未知 action：{}", action)),
    }
}

#[tauri::command]
pub fn agent_wizard_send_key(
    state: tauri::State<'_, AgentWizardState>,
    action: String,
) -> Result<(), String> {
    use std::io::Write;
    let mut g = state.inner.lock().unwrap();
    let inner = g.as_mut().ok_or("向导未在运行")?;
    let data = action_to_bytes(&action)?;
    inner.writer.write_all(&data).map_err(|e| e.to_string())?;
    inner.writer.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agent_wizard_send_keys(
    state: tauri::State<'_, AgentWizardState>,
    actions: Vec<String>,
) -> Result<(), String> {
    use std::io::Write;
    let mut g = state.inner.lock().unwrap();
    let inner = g.as_mut().ok_or("向导未在运行")?;
    for action in &actions {
        let data = action_to_bytes(action)?;
        inner.writer.write_all(&data).map_err(|e| e.to_string())?;
    }
    inner.writer.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kill_agent_wizard(state: tauri::State<'_, AgentWizardState>) -> Result<(), String> {
    let mut g = state.inner.lock().unwrap();
    if let Some(mut inner) = g.take() { inner.kill(); }
    Ok(())
}

#[tauri::command]
pub fn is_agent_wizard_running(state: tauri::State<'_, AgentWizardState>) -> bool {
    state.inner.lock().unwrap().is_some()
}
