use crate::config;

pub fn calc_webview_rect(window: &tauri::Window, right_margin: f64) -> Result<(f64, f64, f64, f64), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let total_w = inner.width as f64 / scale;
    let h = inner.height as f64 / scale;
    let w = (total_w - config::LEFT_PANEL_WIDTH - right_margin).max(0.0);
    Ok((
        config::LEFT_PANEL_WIDTH,
        config::TAB_BAR_HEIGHT,
        w,
        h - config::TAB_BAR_HEIGHT,
    ))
}
