//! 检查并修复 ~/.openclaw/openclaw.json 中 gateway 所需的配置项。
//! 同时提供重启 gateway 的命令。

use serde::Serialize;
use serde_json::Value;
use std::fs;
use tauri::AppHandle;

const REQUIRED_ORIGINS: &[&str] = &["http://localhost:*", "http://127.0.0.1:*"];

fn openclaw_config_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(crate::installer::openclaw_dir(app)?.join("openclaw.json"))
}

/// 检查 gateway 配置是否满足要求。
fn is_config_ok(cfg: &Value) -> bool {
    let gateway = match cfg.get("gateway") {
        Some(g) => g,
        None => return false,
    };

    // controlUi.allowedOrigins 包含必须的两条，且 allowInsecureAuth=true
    let allowed = gateway
        .pointer("/controlUi/allowedOrigins")
        .and_then(|v| v.as_array());
    let origins_ok = match allowed {
        Some(arr) => {
            let strs: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            REQUIRED_ORIGINS.iter().all(|r| strs.contains(r))
        }
        None => false,
    };
    let insecure_ok = gateway
        .pointer("/controlUi/allowInsecureAuth")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // http.endpoints.chatCompletions.enabled = true
    let chat_ok = gateway
        .pointer("/http/endpoints/chatCompletions/enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // http.endpoints.responses.enabled = true
    let responses_ok = gateway
        .pointer("/http/endpoints/responses/enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    origins_ok && insecure_ok && chat_ok && responses_ok
}

/// 将必要配置合并写入，不覆盖用户的其他字段。
fn apply_gateway_config(cfg: &mut Value) {
    let gateway = cfg
        .as_object_mut()
        .unwrap()
        .entry("gateway")
        .or_insert_with(|| serde_json::json!({}));

    // controlUi
    {
        let control_ui = gateway
            .as_object_mut()
            .unwrap()
            .entry("controlUi")
            .or_insert_with(|| serde_json::json!({}));

        // allowedOrigins：合并，不重复
        let origins = control_ui
            .as_object_mut()
            .unwrap()
            .entry("allowedOrigins")
            .or_insert_with(|| serde_json::json!([]));
        let arr = origins.as_array_mut().unwrap();
        for r in REQUIRED_ORIGINS {
            if !arr.iter().any(|v| v.as_str() == Some(r)) {
                arr.push(serde_json::json!(r));
            }
        }

        // allowInsecureAuth
        control_ui
            .as_object_mut()
            .unwrap()
            .entry("allowInsecureAuth")
            .or_insert(serde_json::json!(true));
    }

    // http.endpoints
    {
        let http = gateway
            .as_object_mut()
            .unwrap()
            .entry("http")
            .or_insert_with(|| serde_json::json!({}));
        let endpoints = http
            .as_object_mut()
            .unwrap()
            .entry("endpoints")
            .or_insert_with(|| serde_json::json!({}));
        for key in &["chatCompletions", "responses"] {
            let ep = endpoints
                .as_object_mut()
                .unwrap()
                .entry(*key)
                .or_insert_with(|| serde_json::json!({}));
            ep.as_object_mut()
                .unwrap()
                .entry("enabled")
                .or_insert(serde_json::json!(true));
        }
    }
}

// ── 返回类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GatewayConfigStatus {
    /// 配置原本就正确，无需修改
    pub already_ok: bool,
    /// 配置已修复（写入成功）
    pub fixed: bool,
    /// 是否需要重启 gateway
    pub needs_restart: bool,
    pub error: Option<String>,
}

// ── Tauri commands ───────────────────────────────────────────────────────────

/// 检查 ~/.openclaw/openclaw.json 中 gateway 配置是否满足要求；
/// 如果不满足，自动补写缺失项并返回 fixed=true。
#[tauri::command]
pub fn check_and_fix_gateway_config(app: AppHandle) -> GatewayConfigStatus {
    let path = match openclaw_config_path(&app) {
        Ok(p) => p,
        Err(e) => return GatewayConfigStatus { already_ok: false, fixed: false, needs_restart: false, error: Some(e) },
    };

    // 读取（文件不存在则当作空对象）
    let mut cfg: Value = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or(serde_json::json!({})),
            Err(e) => return GatewayConfigStatus { already_ok: false, fixed: false, needs_restart: false, error: Some(e.to_string()) },
        }
    } else {
        serde_json::json!({})
    };

    if is_config_ok(&cfg) {
        return GatewayConfigStatus { already_ok: true, fixed: false, needs_restart: false, error: None };
    }

    // 修复配置
    if cfg.as_object().is_none() {
        cfg = serde_json::json!({});
    }
    apply_gateway_config(&mut cfg);

    // 写回（保留原有格式缩进）
    let pretty = match serde_json::to_string_pretty(&cfg) {
        Ok(s) => s,
        Err(e) => return GatewayConfigStatus { already_ok: false, fixed: false, needs_restart: false, error: Some(e.to_string()) },
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(e) = fs::write(&path, pretty) {
        return GatewayConfigStatus { already_ok: false, fixed: false, needs_restart: false, error: Some(e.to_string()) };
    }

    GatewayConfigStatus { already_ok: false, fixed: true, needs_restart: true, error: None }
}

/// 执行 openclaw gateway 子命令的通用实现。
fn run_openclaw_gateway_cmd(#[allow(unused)] app: &AppHandle, subcmd: &str) -> Result<(), String> {
    let full_cmd = if subcmd.is_empty() {
        "openclaw gateway".to_string()
    } else {
        format!("openclaw gateway {}", subcmd)
    };

    #[cfg(not(target_os = "windows"))]
    {
        let mut candidates: Vec<(std::path::PathBuf, bool)> = Vec::new();
        if let Ok(s) = std::env::var("SHELL") {
            let p = std::path::PathBuf::from(&s);
            if p.exists() {
                candidates.push((p, true));
            }
        }
        for sh in &["/bin/zsh", "/bin/bash"] {
            let p = std::path::PathBuf::from(sh);
            if p.exists() && !candidates.iter().any(|(q, _)| q == &p) {
                candidates.push((p, true));
            }
        }
        candidates.push((std::path::PathBuf::from("/bin/sh"), false));

        let mut last_err = String::from("未找到可用 shell");
        for (shell, login) in &candidates {
            let mut cmd = std::process::Command::new(shell);
            if *login {
                cmd.args(["-l", "-c", &full_cmd]);
            } else {
                cmd.args(["-c", &full_cmd]);
            }
            match cmd.output() {
                Ok(o) if o.status.success() => return Ok(()),
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    let msg = if !stderr.is_empty() { stderr } else { stdout };
                    if msg.contains("command not found") || msg.contains("not found") {
                        last_err = msg;
                        continue;
                    }
                    return Err(msg);
                }
                Err(e) => {
                    last_err = format!("无法执行 {}: {}", shell.display(), e);
                }
            }
        }
        Err(last_err)
    }

    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(["/C", &full_cmd]);
        cmd.env("PATH", crate::installer::build_openclaw_env_path(app));
        if let Some(ref safe_home) = crate::installer::safe_home_for_openclaw() {
            cmd.env("HOME", safe_home);
        }
        if let Some(ref prefix) = crate::installer::safe_npm_prefix() {
            cmd.env("NPM_CONFIG_PREFIX", prefix);
        }
        match cmd.output() {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                Err(if !stderr.is_empty() { stderr } else { stdout })
            }
            Err(e) => Err(format!("无法执行命令：{}", e)),
        }
    }
}

/// 首次启动 gateway（openclaw onboard 完成后使用）。
#[tauri::command]
pub fn start_openclaw_gateway(app: AppHandle) -> Result<(), String> {
    run_openclaw_gateway_cmd(&app, "")
}

/// 重启已在运行的 gateway。
#[tauri::command]
pub fn restart_openclaw_gateway(app: AppHandle) -> Result<(), String> {
    run_openclaw_gateway_cmd(&app, "restart")
}
