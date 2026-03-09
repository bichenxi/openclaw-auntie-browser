mod api;
mod app;
mod config;
mod openclaw;
mod openclaw_http;
mod openclaw_process;
mod profile;
mod sidecar;
mod skills;
mod webview;
mod xhs;

use api::{set_active_tab_label, set_ai_paused, ActiveTabLabel, AiPaused, PendingEvalResult, PendingSnapshot};
use app::{emit_stream_item, simulate_stream};
use openclaw::{openclaw_connect, openclaw_disconnect, openclaw_send_chat, OpenClawState};
use openclaw_http::{check_openclaw_alive, openclaw_send_v1};
use profile::{get_current_profile, set_current_profile};
use tauri::generate_handler;
use webview::commands::{
    close_webview, create_tab_webview, eval_in_webview, get_dom_snapshot, hide_webview,
    resize_all_webviews, show_webview,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(OpenClawState::default())
        .manage(ActiveTabLabel(std::sync::Mutex::new(None)))
        .manage(PendingSnapshot(std::sync::Mutex::new(None)))
        .manage(PendingEvalResult(std::sync::Mutex::new(None)))
        .manage(AiPaused::default())
        .manage(openclaw_process::OpenClawProcess::default())
        .setup(|app| {
            api::spawn_http_server(app.handle().clone());
            Ok(())
        })
        .invoke_handler(generate_handler![
            app::greet,
            app::on_webview_click,
            emit_stream_item,
            simulate_stream,
            get_current_profile,
            set_current_profile,
            openclaw_connect,
            openclaw_send_chat,
            openclaw_disconnect,
            openclaw_send_v1,
            check_openclaw_alive,
            sidecar::test_sidecar,
            openclaw_process::start_openclaw,
            openclaw_process::stop_openclaw,
            openclaw_process::is_openclaw_running,
            create_tab_webview,
            show_webview,
            hide_webview,
            close_webview,
            resize_all_webviews,
            eval_in_webview,
            get_dom_snapshot,
            set_active_tab_label,
            set_ai_paused,
            skills::list_skills,
            skills::read_skill_file,
            skills::write_skill_file,
            skills::create_skill,
            skills::delete_skill,
            skills::delete_skill_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
