//! HTTP API on 127.0.0.1:18790 for OpenClaw to control the browser.

use std::sync::Mutex;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;

use crate::webview::commands;

// ── Shared state ──────────────────────────────────────────────────────────────

/// Which webview label is currently active (the one shown in the tab bar).
pub struct ActiveTabLabel(pub Mutex<Option<String>>);

/// Oneshot sender set when /snapshot is waiting for a DOM snapshot response.
pub struct PendingSnapshot(pub Mutex<Option<oneshot::Sender<String>>>);

/// Oneshot sender set when /eval is waiting for a JS eval result.
pub struct PendingEvalResult(pub Mutex<Option<oneshot::Sender<String>>>);

/// Whether the AI is paused (human in control). When true, all browser-mutation
/// endpoints return 503 so the OpenClaw agent knows to stop issuing commands.
pub struct AiPaused(pub std::sync::atomic::AtomicBool);

impl Default for AiPaused {
    fn default() -> Self {
        AiPaused(std::sync::atomic::AtomicBool::new(false))
    }
}

/// Tauri command: frontend calls this to pause/resume AI browser control.
#[tauri::command]
pub fn set_ai_paused(app: AppHandle, paused: bool) {
    if let Some(state) = app.try_state::<AiPaused>() {
        state.0.store(paused, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Returns Err with 503 if AI is currently paused.
pub(crate) fn check_paused(app: &AppHandle) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if let Some(state) = app.try_state::<AiPaused>() {
        if state.0.load(std::sync::atomic::Ordering::SeqCst) {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "AI is paused. Human operator is in control. Please wait for resume.".to_string(),
                }),
            ));
        }
    }
    Ok(())
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NavigateBody {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ClickBody {
    pub selector: String,
}

#[derive(Debug, Deserialize)]
pub struct TypeBody {
    pub selector: String,
    pub text: String,
    /// If true, appends to current value instead of replacing it.
    #[serde(default)]
    pub append: bool,
}

#[derive(Debug, Deserialize)]
pub struct ScrollBody {
    /// Scroll a specific element into view (takes priority over x/y).
    pub selector: Option<String>,
    /// Absolute scroll X position (pixels). Defaults to current.
    pub x: Option<i32>,
    /// Absolute scroll Y position (pixels). Defaults to current.
    pub y: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SelectBody {
    pub selector: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct EvalBody {
    pub script: String,
}

#[derive(Debug, Deserialize)]
pub struct HighlightBody {
    pub selector: String,
}

#[derive(Debug, Deserialize)]
pub struct WaitBody {
    pub selector: String,
    /// Max milliseconds to wait. Defaults to 10000 (10 s).
    #[serde(default = "default_wait_timeout")]
    pub timeout: u64,
}
fn default_wait_timeout() -> u64 { 10000 }

#[derive(Debug, Deserialize)]
pub struct ExtractBody {
    pub selector: String,
    #[serde(default = "default_extract_limit")]
    pub limit: u32,
}
fn default_extract_limit() -> u32 { 50 }

#[derive(Debug, Deserialize)]
pub struct ExtractTextBody {
    /// CSS selector for the container to read from. Omit or null → read from <body>.
    pub selector: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EvalResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Payload for api_open_tab event when /navigate is called with no active tab.
#[derive(Clone, Debug, Serialize)]
pub struct ApiOpenTabPayload {
    pub url: String,
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Tauri command: frontend calls this when the active tab (or home) changes.
#[tauri::command]
pub fn set_active_tab_label(app: AppHandle, label: Option<String>) {
    if let Some(state) = app.try_state::<ActiveTabLabel>() {
        if let Ok(mut guard) = state.0.lock() {
            *guard = label;
        }
    }
}

pub(crate) fn get_active_label(app: &AppHandle) -> Option<String> {
    let state = app.try_state::<ActiveTabLabel>()?;
    let guard = state.0.lock().ok()?;
    (*guard).clone()
}

pub(crate) fn no_active_tab() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "No active tab".to_string(),
        }),
    )
}

pub(crate) fn internal_err(e: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e }))
}

/// JSON-encodes a Rust &str into a JS string literal (e.g. `"hello"`, `"a\\nb"`).
/// Safe to embed directly inside JS source.
pub(crate) fn to_js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

// ── Async helpers ─────────────────────────────────────────────────────────────

/// Triggers a DOM snapshot and waits for the result (up to 10 s).
async fn get_dom_snapshot_sync(app: AppHandle, label: String) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    {
        let pending = app
            .try_state::<PendingSnapshot>()
            .ok_or_else(|| "PendingSnapshot state not found".to_string())?;
        let mut guard = pending.0.lock().map_err(|e| e.to_string())?;
        *guard = Some(tx);
    }
    commands::trigger_dom_snapshot(&app, &label)?;
    tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .map_err(|_| "DOM snapshot timeout".to_string())?
        .map_err(|_| "DOM snapshot channel closed".to_string())
}

/// Evaluates `script` in the webview and waits for the JSON result (up to 10 s).
/// Result is `{"ok":true,"value":...}` or `{"ok":false,"error":"..."}`.
pub(crate) async fn eval_with_result(app: AppHandle, label: String, script: String) -> Result<String, String> {
    let (tx, rx) = oneshot::channel();
    {
        let pending = app
            .try_state::<PendingEvalResult>()
            .ok_or_else(|| "PendingEvalResult state not found".to_string())?;
        let mut guard = pending.0.lock().map_err(|e| e.to_string())?;
        *guard = Some(tx);
    }
    commands::trigger_eval(&app, &label, &script)?;
    tokio::time::timeout(Duration::from_secs(10), rx)
        .await
        .map_err(|_| "eval timeout".to_string())?
        .map_err(|_| "eval channel closed".to_string())
}

// ── Route handlers ────────────────────────────────────────────────────────────

/// POST /navigate  { "url": "https://..." }
/// Navigates the active tab, or emits api_open_tab to open a new one.
async fn handle_navigate(
    State(app): State<AppHandle>,
    Json(body): Json<NavigateBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let url = body.url;
    if let Some(label) = get_active_label(&app) {
        let script = format!(
            "window.location.href = {};",
            serde_json::to_string(&url).map_err(|e| internal_err(e.to_string()))?
        );
        commands::eval_in_webview(app, label, script)
            .await
            .map_err(internal_err)?;
    } else {
        let _ = app.emit("api_open_tab", ApiOpenTabPayload { url });
    }
    Ok(StatusCode::NO_CONTENT)
}

/// GET /snapshot
/// Returns full page context: { meta: { url, title, viewport, scroll }, elements: [...] }
/// Each element has: id, role, tag, text, selector, rect, inViewport, + type-specific fields.
async fn handle_snapshot(
    State(app): State<AppHandle>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let snapshot_str = get_dom_snapshot_sync(app, label)
        .await
        .map_err(internal_err)?;
    let value: serde_json::Value = serde_json::from_str(&snapshot_str).unwrap_or_else(|_| {
        serde_json::json!({ "meta": {}, "elements": [], "_parseError": true })
    });
    Ok(Json(value))
}

/// POST /click  { "selector": "CSS selector" }
async fn handle_click(
    State(app): State<AppHandle>,
    Json(body): Json<ClickBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let sel_js = to_js_str(&body.selector);
    let script = format!(
        "(function(){{var el=document.querySelector({s});if(el)el.click();}})();",
        s = sel_js
    );
    commands::eval_in_webview(app, label, script)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /type  { "selector": "...", "text": "...", "append": false }
/// Fills an input or textarea, triggering React/Vue reactivity events.
async fn handle_type(
    State(app): State<AppHandle>,
    Json(body): Json<TypeBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let sel_js = to_js_str(&body.selector);
    let text_js = to_js_str(&body.text);
    let append = body.append;
    let script = format!(
        r#"(function(){{
var el=document.querySelector({sel});
if(!el)return;
el.focus();
var proto=el instanceof HTMLTextAreaElement?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
var desc=Object.getOwnPropertyDescriptor(proto,'value');
var newVal={append}?(el.value||'')+{text}:{text};
if(desc&&desc.set){{desc.set.call(el,newVal);}}else{{el.value=newVal;}}
['input','change'].forEach(function(t){{el.dispatchEvent(new Event(t,{{bubbles:true}}));}});
}})();"#,
        sel = sel_js,
        text = text_js,
        append = append
    );
    commands::eval_in_webview(app, label, script)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /scroll  { "selector": "..." }  — scrolls element into view
/// POST /scroll  { "y": 800 }           — absolute scroll position
/// POST /scroll  { "x": 0, "y": 0 }    — scroll to position
async fn handle_scroll(
    State(app): State<AppHandle>,
    Json(body): Json<ScrollBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let script = if let Some(selector) = body.selector {
        let sel_js = to_js_str(&selector);
        format!(
            "var __el=document.querySelector({s});if(__el)__el.scrollIntoView({{behavior:'smooth',block:'center'}});",
            s = sel_js
        )
    } else {
        let x = body.x.unwrap_or(0);
        let y = body.y.unwrap_or(0);
        format!(
            "window.scrollTo({{top:{y},left:{x},behavior:'smooth'}});",
            x = x,
            y = y
        )
    };
    commands::eval_in_webview(app, label, script)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /select  { "selector": "select#city", "value": "beijing" }
async fn handle_select(
    State(app): State<AppHandle>,
    Json(body): Json<SelectBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let sel_js = to_js_str(&body.selector);
    let val_js = to_js_str(&body.value);
    let script = format!(
        "(function(){{var el=document.querySelector({s});if(el){{el.value={v};el.dispatchEvent(new Event('change',{{bubbles:true}}));}}}})();",
        s = sel_js,
        v = val_js
    );
    commands::eval_in_webview(app, label, script)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /eval  { "script": "document.title" }
/// Evaluates arbitrary JS and returns the result.
/// Response: { "ok": true, "value": <any> } or { "ok": false, "error": "..." }
async fn handle_eval(
    State(app): State<AppHandle>,
    Json(body): Json<EvalBody>,
) -> Result<Json<EvalResponse>, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let result_str = eval_with_result(app, label, body.script)
        .await
        .map_err(internal_err)?;
    let response: EvalResponse = serde_json::from_str(&result_str).unwrap_or(EvalResponse {
        ok: false,
        value: None,
        error: Some(format!("failed to parse eval result: {}", result_str)),
    });
    Ok(Json(response))
}

/// GET /page-info
/// Returns { "url": "...", "title": "...", "readyState": "complete" } quickly.
async fn handle_page_info(
    State(app): State<AppHandle>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let result_str = eval_with_result(
        app,
        label,
        "JSON.stringify({url:location.href,title:document.title,readyState:document.readyState})".to_string(),
    )
    .await
    .map_err(internal_err)?;

    let eval_resp: EvalResponse =
        serde_json::from_str(&result_str).map_err(|e| internal_err(e.to_string()))?;
    if eval_resp.ok {
        let info = match eval_resp.value {
            Some(serde_json::Value::String(s)) => {
                serde_json::from_str::<serde_json::Value>(&s).unwrap_or_default()
            }
            Some(v) => v,
            None => serde_json::Value::Object(Default::default()),
        };
        Ok(Json(info))
    } else {
        Err(internal_err(eval_resp.error.unwrap_or_default()))
    }
}

/// POST /highlight  { "selector": "..." }
/// Draws a red border around the matched element for 2.5 s (visual feedback).
async fn handle_highlight(
    State(app): State<AppHandle>,
    Json(body): Json<HighlightBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let sel_js = to_js_str(&body.selector);
    let script = format!(
        "window.__clawBridge&&window.__clawBridge.highlight({s});",
        s = sel_js
    );
    commands::eval_in_webview(app, label, script)
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /wait  { "selector": "...", "timeout": 10000 }
/// Polls every 500 ms until the selector matches a visible element (or timeout).
async fn handle_wait(
    State(app): State<AppHandle>,
    Json(body): Json<WaitBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let deadline = tokio::time::Instant::now() + Duration::from_millis(body.timeout);
    let check_script = format!(
        "!!document.querySelector({})",
        to_js_str(&body.selector)
    );
    loop {
        let result_str = eval_with_result(app.clone(), label.clone(), check_script.clone())
            .await
            .map_err(internal_err)?;
        if let Ok(resp) = serde_json::from_str::<EvalResponse>(&result_str) {
            if resp.ok && resp.value == Some(serde_json::Value::Bool(true)) {
                return Ok(StatusCode::NO_CONTENT);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err((
                StatusCode::REQUEST_TIMEOUT,
                Json(ErrorResponse {
                    error: format!("element '{}' not found within {} ms", body.selector, body.timeout),
                }),
            ));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// POST /extract  { "selector": ".news-item", "limit": 30 }
/// Returns structured items: [{ text, href?, src?, selector }].
async fn handle_extract(
    State(app): State<AppHandle>,
    Json(body): Json<ExtractBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let script = format!(
        "JSON.stringify(window.__clawBridge.extractContent({},{}))",
        to_js_str(&body.selector),
        body.limit
    );
    let result_str = eval_with_result(app, label, script)
        .await
        .map_err(internal_err)?;
    let eval_resp: EvalResponse =
        serde_json::from_str(&result_str).map_err(|e| internal_err(e.to_string()))?;
    if eval_resp.ok {
        let items = match eval_resp.value {
            Some(serde_json::Value::String(s)) => {
                serde_json::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::json!([]))
            }
            Some(v) => v,
            None => serde_json::json!([]),
        };
        Ok(Json(serde_json::json!({ "items": items })))
    } else {
        Err(internal_err(eval_resp.error.unwrap_or_default()))
    }
}

/// POST /extract-text  { "selector": "article" }
/// Returns the readable text content of the matched region.
async fn handle_extract_text(
    State(app): State<AppHandle>,
    Json(body): Json<ExtractTextBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    let sel_arg = match &body.selector {
        Some(s) => to_js_str(s),
        None => "null".to_string(),
    };
    let script = format!(
        "window.__clawBridge.extractText({})",
        sel_arg
    );
    let result_str = eval_with_result(app, label, script)
        .await
        .map_err(internal_err)?;
    let eval_resp: EvalResponse =
        serde_json::from_str(&result_str).map_err(|e| internal_err(e.to_string()))?;
    if eval_resp.ok {
        let text = match eval_resp.value {
            Some(serde_json::Value::String(s)) => s,
            Some(v) => v.to_string(),
            None => String::new(),
        };
        Ok(Json(serde_json::json!({ "text": text })))
    } else {
        Err(internal_err(eval_resp.error.unwrap_or_default()))
    }
}

/// POST /back — browser history back
async fn handle_back(
    State(app): State<AppHandle>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    commands::eval_in_webview(app, label, "history.back()".to_string())
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /forward — browser history forward
async fn handle_forward(
    State(app): State<AppHandle>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    check_paused(&app)?;
    let label = get_active_label(&app).ok_or_else(no_active_tab)?;
    commands::eval_in_webview(app, label, "history.forward()".to_string())
        .await
        .map_err(internal_err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Server bootstrap ──────────────────────────────────────────────────────────

pub fn spawn_http_server(app: AppHandle) {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("create tokio runtime for API");
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:18790")
                .await
                .expect("bind 127.0.0.1:18790");
            let router = Router::new()
                .route("/navigate", post(handle_navigate))
                .route("/snapshot", get(handle_snapshot))
                .route("/click", post(handle_click))
                .route("/type", post(handle_type))
                .route("/scroll", post(handle_scroll))
                .route("/select", post(handle_select))
                .route("/eval", post(handle_eval))
                .route("/page-info", get(handle_page_info))
                .route("/highlight", post(handle_highlight))
                .route("/wait", post(handle_wait))
                .route("/extract", post(handle_extract))
                .route("/extract-text", post(handle_extract_text))
                .route("/back", post(handle_back))
                .route("/forward", post(handle_forward))
                .with_state(app_clone)
                .layer(tower_http::cors::CorsLayer::permissive());
            axum::serve(listener, router).await.expect("serve API");
        });
    });
}
