// 前端 header 分两层：
//   - 身份行（Profile row）: py-1.5 + 按钮 ≈ 38px
//   - TabBar:               h-11 = 44px
// 合计约 82px。
// macOS: Overlay 标题栏约 44px，webview y 相对整窗，取 128 留安全余量。
// Windows: 保留原生标题栏（约 32px），取 114。
#[cfg(target_os = "macos")]
pub const TAB_BAR_HEIGHT: f64 = 128.0;
#[cfg(target_os = "windows")]
pub const TAB_BAR_HEIGHT: f64 = 114.0;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub const TAB_BAR_HEIGHT: f64 = 114.0;

pub const LEFT_PANEL_WIDTH: f64 = 0.0;

// Chromium-compatible User-Agent for child webviews.
#[cfg(target_os = "windows")]
pub const CHROME_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
#[cfg(target_os = "macos")]
pub const CHROME_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub const CHROME_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
