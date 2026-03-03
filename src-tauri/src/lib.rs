use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl};

// 44px 为前端 TabBar 高度；macOS 下子 webview 的 y 可能相对整窗（含标题栏），多留余量避免遮挡
const TAB_BAR_HEIGHT: f64 = 88.0;

fn calc_webview_rect(window: &tauri::Window) -> Result<(f64, f64, f64, f64), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let w = inner.width as f64 / scale;
    let h = inner.height as f64 / scale;
    Ok((0.0, TAB_BAR_HEIGHT, w, h - TAB_BAR_HEIGHT))
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! OpenClaw Auntie Browser ready.", name)
}

#[tauri::command]
async fn create_tab_webview(app: AppHandle, label: String, url: String) -> Result<(), String> {
    let window = app.get_window("main").ok_or("main window not found")?;

    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    let (x, y, w, h) = calc_webview_rect(&window)?;

    window
        .add_child(
            tauri::webview::WebviewBuilder::new(&label, WebviewUrl::External(parsed_url)),
            LogicalPosition::new(x, y),
            LogicalSize::new(w, h),
        )
        .map_err(|e: tauri::Error| e.to_string())?;

    // 先隐藏，等前端展示完加载动画后再 show
    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.hide();
    }
    Ok(())
}

#[tauri::command]
async fn show_webview(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.show().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn hide_webview(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn close_webview(app: AppHandle, label: String) -> Result<(), String> {
    let wv = app
        .get_webview(&label)
        .ok_or(format!("webview '{}' not found", label))?;
    wv.close().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn resize_all_webviews(app: AppHandle, labels: Vec<String>) -> Result<(), String> {
    let window = app.get_window("main").ok_or("main window not found")?;

    let (x, y, w, h) = calc_webview_rect(&window)?;

    for label in labels {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.set_position(LogicalPosition::new(x, y));
            let _ = wv.set_size(LogicalSize::new(w, h));
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            create_tab_webview,
            show_webview,
            hide_webview,
            close_webview,
            resize_all_webviews,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
