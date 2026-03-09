//! OpenClaw HTTP API：POST /v1/responses（Bearer + x-openclaw-session-key），流式输出通过 stream-item 推送到前端。

use futures_util::StreamExt;
use serde::Deserialize;
use tauri::{AppHandle, Emitter};

const DEFAULT_API_BASE: &str = "http://127.0.0.1:18789";

fn emit_delta(app: &AppHandle, msg_type: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    let _ = app.emit(
        "stream-item",
        serde_json::json!({ "type": msg_type, "text": text }),
    );
}

fn emit_done(app: &AppHandle) {
    let _ = app.emit("stream-done", serde_json::Value::Null);
}

/// 从一个 SSE data 行中提取 (消息类型, delta文本)。
/// event_type 是上方 `event:` 行的值（可能为空）。
fn parse_sse_data(event_type: &str, data: &str) -> Option<(&'static str, String)> {
    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    let obj = value.as_object()?;

    // 优先用 JSON 内的 `type` 字段判断，回退到 SSE event 头
    let json_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or(event_type);

    // 判断消息分类
    let msg_type: &'static str =
        if json_type.contains("function_call") || json_type.contains("tool_call") {
            "tool"
        } else {
            "thought"
        };

    // ① Responses API：顶层 delta 字段为字符串
    if let Some(delta) = obj.get("delta").and_then(|v| v.as_str()) {
        if !delta.is_empty() {
            return Some((msg_type, delta.to_string()));
        }
    }

    // ② Chat Completions API：choices[0].delta.content
    if let Some(choices) = obj.get("choices").and_then(|v| v.as_array()) {
        if let Some(choice) = choices.first() {
            if let Some(delta_obj) = choice.get("delta").and_then(|v| v.as_object()) {
                if let Some(content) = delta_obj.get("content").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        return Some((msg_type, content.to_string()));
                    }
                }
            }
        }
    }

    // ③ 简单 text / content 字段
    if let Some(t) = obj.get("text").and_then(|v| v.as_str()) {
        if !t.is_empty() {
            return Some((msg_type, t.to_string()));
        }
    }
    if let Some(t) = obj.get("content").and_then(|v| v.as_str()) {
        if !t.is_empty() {
            return Some((msg_type, t.to_string()));
        }
    }

    None
}

#[derive(Debug, Deserialize)]
pub struct OpenclawV1Params {
    #[serde(default)]
    pub base_url: Option<String>,
    pub token: Option<String>,
    #[serde(default)]
    pub session_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub input: String,
    #[serde(default = "default_true")]
    pub stream: bool,
}
fn default_true() -> bool {
    true
}

/// 检查 OpenClaw HTTP 服务是否在线（任意 HTTP 响应均视为在线）。
#[tauri::command]
pub async fn check_openclaw_alive(base_url: Option<String>) -> bool {
    let base = base_url.unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    let url = format!("{}/", base.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client.get(&url).send().await.is_ok()
}

#[tauri::command]
pub async fn openclaw_send_v1(app: AppHandle, params: OpenclawV1Params) -> Result<(), String> {
    let base = params
        .base_url
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    let url = format!("{}/v1/responses", base.trim_end_matches('/'));

    let token = params
        .token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| {
            std::env::var("OPENCLAW_BEARER_TOKEN")
                .ok()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
        })
        .ok_or("缺少 token：请传入 token 或设置环境变量 OPENCLAW_BEARER_TOKEN")?;

    let session_key = params
        .session_key
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("OPENCLAW_SESSION_KEY")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "agent:main:main2".to_string());

    let model = params
        .model
        .or_else(|| std::env::var("OPENCLAW_MODEL").ok())
        .unwrap_or_else(|| "minimax-cn/MiniMax-M2.5".to_string());

    let body = serde_json::json!({
        "model": model,
        "input": params.input,
        "stream": params.stream,
    });

    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("x-openclaw-session-key", &session_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, err_body));
    }

    // ── 非流式：读取完整响应，解析后一次性发送 ──────────────────────────
    if !params.stream {
        let text = response.text().await.map_err(|e| e.to_string())?;
        if let Some((msg_type, t)) = parse_sse_data("", &text) {
            emit_delta(&app, msg_type, &t);
        } else if !text.trim().is_empty() {
            emit_delta(&app, "thought", text.trim());
        }
        emit_done(&app);
        return Ok(());
    }

    // ── 流式：按 SSE 规范逐块解析 ────────────────────────────────────────
    // SSE 格式：
    //   event: <event-type>\n      ← 可选
    //   data: <json>\n
    //   \n                         ← 空行标志一个事件结束
    let mut byte_buf = Vec::<u8>::new();
    let mut current_event_type = String::new();
    let mut byte_stream = response.bytes_stream();

    while let Some(chunk) = byte_stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        byte_buf.extend_from_slice(&chunk);

        // 逐行切分（保留完整行）
        while let Some(pos) = byte_buf.iter().position(|&b| b == b'\n') {
            let raw = byte_buf.drain(..=pos).collect::<Vec<_>>();
            let line = String::from_utf8_lossy(&raw).trim_end_matches('\r').trim().to_string();

            if line.is_empty() {
                // 空行 = 当前 SSE 事件块结束，重置 event type
                current_event_type.clear();
            } else if let Some(ev) = line.strip_prefix("event:") {
                current_event_type = ev.trim().to_string();
            } else if let Some(data) = line.strip_prefix("data:") {
                if let Some((msg_type, text)) = parse_sse_data(&current_event_type, data) {
                    emit_delta(&app, msg_type, &text);
                }
            }
            // id: / retry: 行忽略
        }
    }

    // 处理缓冲区中最后一行（无尾部换行时）
    if !byte_buf.is_empty() {
        let line = String::from_utf8_lossy(&byte_buf).trim().to_string();
        if let Some(data) = line.strip_prefix("data:") {
            if let Some((msg_type, text)) = parse_sse_data(&current_event_type, data) {
                emit_delta(&app, msg_type, &text);
            }
        }
    }

    emit_done(&app);
    Ok(())
}
