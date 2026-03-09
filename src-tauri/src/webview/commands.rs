use base64::Engine;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl};

use crate::config;
use crate::profile;
use crate::webview::rect;

#[tauri::command]
pub async fn create_tab_webview(app: AppHandle, label: String, url: String, right_margin: f64) -> Result<(), String> {
    let window = app.get_window("main").ok_or("main window not found")?;

    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    let (x, y, w, h) = rect::calc_webview_rect(&window, right_margin)?;
    let stealth_js = include_str!("../stealth.js");
    let bridge_js = include_str!("../bridge.js");
    let xhs_js = include_str!("../xhs.js");
    let init_script = format!(
        "{}\n{};\n{}\nwindow.__clawBridgeLabel={:?};",
        stealth_js,
        bridge_js.trim_end().trim_end_matches(';'),
        xhs_js,
        label
    );

    let app_emit = app.clone();
    let nav_handler = move |nav_url: &url::Url| {
        if nav_url.scheme() != "claw" {
            return true;
        }
        match nav_url.host_str() {
            Some("webview-click") => {
                let mut label_val = String::new();
                let mut x_val = 0i32;
                let mut y_val = 0i32;
                let mut tag_val = String::new();
                for (k, v) in nav_url.query_pairs() {
                    match k.as_ref() {
                        "label" => label_val = v.to_string(),
                        "x" => x_val = v.parse().unwrap_or(0),
                        "y" => y_val = v.parse().unwrap_or(0),
                        "tag" => tag_val = v.to_string(),
                        _ => {}
                    }
                }
                println!(
                    "Webview '{}' clicked at ({}, {}) on {}",
                    label_val, x_val, y_val, tag_val
                );
                false
            }
            Some("dom-snapshot") => {
                if let Some(frag) = nav_url.fragment() {
                    let b64 = frag.replace('-', "+").replace('_', "/");
                    if let Ok(decoded) =
                        base64::engine::general_purpose::STANDARD.decode(b64.as_bytes())
                    {
                        if let Ok(s) = String::from_utf8(decoded) {
                            if let Some(pending) =
                                app_emit.try_state::<crate::api::PendingSnapshot>()
                            {
                                if let Ok(mut guard) = pending.0.try_lock() {
                                    if let Some(tx) = guard.take() {
                                        let _ = tx.send(s.clone());
                                    }
                                }
                            }
                            let _ = app_emit.emit("dom-snapshot", &s);
                        }
                    }
                }
                false
            }
            Some("eval-result") => {
                if let Some(frag) = nav_url.fragment() {
                    let b64 = frag.replace('-', "+").replace('_', "/");
                    if let Ok(decoded) =
                        base64::engine::general_purpose::STANDARD.decode(b64.as_bytes())
                    {
                        if let Ok(s) = String::from_utf8(decoded) {
                            if let Some(pending) =
                                app_emit.try_state::<crate::api::PendingEvalResult>()
                            {
                                if let Ok(mut guard) = pending.0.try_lock() {
                                    if let Some(tx) = guard.take() {
                                        let _ = tx.send(s);
                                    }
                                }
                            }
                        }
                    }
                }
                false
            }
            _ => true,
        }
    };

    let data_dir: PathBuf = profile::profile_webview_data_dir(&app)?;
    let builder = tauri::webview::WebviewBuilder::new(&label, WebviewUrl::External(parsed_url))
        .user_agent(config::CHROME_USER_AGENT)
        .initialization_script(&init_script)
        .on_navigation(nav_handler)
        .data_directory(data_dir);

    window
        .add_child(builder, LogicalPosition::new(x, y), LogicalSize::new(w, h))
        .map_err(|e: tauri::Error| e.to_string())?;

    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.hide();
    }
    Ok(())
}

#[tauri::command]
pub async fn show_webview(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.show().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn hide_webview(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn close_webview(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.close().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn resize_all_webviews(app: AppHandle, labels: Vec<String>, right_margin: f64) -> Result<(), String> {
    let window = app.get_window("main").ok_or("main window not found")?;
    let (x, y, w, h) = rect::calc_webview_rect(&window, right_margin)?;

    for label in labels {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.set_position(LogicalPosition::new(x, y));
            let _ = wv.set_size(LogicalSize::new(w, h));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn eval_in_webview(app: AppHandle, label: String, script: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.eval(&script).map_err(|e| e.to_string())?;
    Ok(())
}

// Uses getPageContext() (rich: meta + elements) instead of the old getSimplifiedDOM().
const DOM_SNAPSHOT_SCRIPT: &str = r#"
(function(){
  var ctx = window.__clawBridge && window.__clawBridge.getPageContext
    ? window.__clawBridge.getPageContext()
    : { meta: { url: location.href, title: document.title,
                viewport: { w: innerWidth, h: innerHeight },
                scroll: { y: 0, maxY: 0 } },
        elements: [] };
  var json = JSON.stringify(ctx);
  var b64 = btoa(unescape(encodeURIComponent(json))).replace(/\+/g,'-').replace(/\//g,'_');
  window.location.assign('claw://dom-snapshot#' + b64);
})();
"#;

/// Called by the HTTP API to trigger snapshot; result is sent via PendingSnapshot oneshot.
pub fn trigger_dom_snapshot(app: &AppHandle, label: &str) -> Result<(), String> {
    let wv = app
        .get_webview(label)
        .ok_or_else(|| format!("webview '{}' not found", label))?;
    wv.eval(DOM_SNAPSHOT_SCRIPT).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_dom_snapshot(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.eval(DOM_SNAPSHOT_SCRIPT).map_err(|e| e.to_string())?;
    Ok(())
}

/// Runs `script` inside the webview and sends the result back via claw://eval-result.
/// The result is received by the PendingEvalResult oneshot in api.rs.
pub fn trigger_eval(app: &AppHandle, label: &str, script: &str) -> Result<(), String> {
    let wv = app
        .get_webview(label)
        .ok_or_else(|| format!("webview '{}' not found", label))?;
    let script_json = serde_json::to_string(script).map_err(|e| e.to_string())?;
    // Wraps user script: eval(__s) → JSON result → claw://eval-result#<b64>
    let wrapper = format!(
        r#"(function(__s){{
try{{var __r=eval(__s);var __o=JSON.stringify({{ok:true,value:__r!==undefined?__r:null}});}}
catch(e){{var __o=JSON.stringify({{ok:false,error:e.message}});}}
var __b=btoa(unescape(encodeURIComponent(__o))).replace(/\+/g,'-').replace(/\//g,'_');
window.location.assign('claw://eval-result#'+__b);}})({});"#,
        script_json
    );
    wv.eval(&wrapper).map_err(|e| e.to_string())?;
    Ok(())
}
