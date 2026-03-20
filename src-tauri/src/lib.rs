mod agents;
mod api;
mod flows;
mod app;
mod config;
mod configure;
mod gateway;
mod installer;
mod openclaw;
mod openclaw_http;
mod openclaw_process;
mod profile;
mod skills;
mod webview;

use api::{
    set_active_tab_label, set_ai_paused, ActiveTabLabel, AiPaused, PendingEvalResult,
    PendingSnapshot,
};
use app::{emit_stream_item, simulate_stream};
use installer::InstallerState;
use openclaw::{openclaw_connect, openclaw_disconnect, openclaw_send_chat, OpenClawState};
use openclaw_http::{check_openclaw_alive, openclaw_send_completions, openclaw_send_v1};
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
        .manage(InstallerState::default())
        .manage(configure::OnboardPtyState::default())
        .manage(configure::OnboardWizardState::default())
        .manage(agents::AgentWizardState::default())
        .manage(flows::FlowRuntimeState::default())
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
            openclaw_send_completions,
            check_openclaw_alive,
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
            skills::list_workspaces,
            skills::list_skills,
            skills::read_skill_file,
            skills::write_skill_file,
            skills::create_skill,
            skills::delete_skill,
            skills::delete_skill_file,
            skills::check_builtin_skill_installed,
            skills::install_builtin_skill,
            skills::get_openclaw_gateway_token,
            skills::sync_skills_to_config,
            installer::start_install,
            installer::cancel_install,
            installer::check_openclaw_installed,
            installer::detect_environment,
            installer::full_uninstall,
            configure::run_onboard,
            configure::start_onboard_pty,
            configure::write_onboard_stdin,
            configure::kill_onboard_pty,
            configure::is_onboard_pty_running,
            configure::start_onboard_wizard,
            configure::wizard_send_key,
            configure::wizard_send_keys,
            configure::kill_onboard_wizard,
            configure::is_onboard_wizard_running,
            configure::is_elevated,
            configure::restart_elevated,
            gateway::check_and_fix_gateway_config,
            gateway::start_openclaw_gateway,
            gateway::restart_openclaw_gateway,
            agents::list_agents,
            agents::list_agent_files,
            agents::read_agent_file,
            agents::write_agent_file,
            agents::start_agent_add_wizard,
            agents::agent_wizard_send_key,
            agents::agent_wizard_send_keys,
            agents::kill_agent_wizard,
            agents::is_agent_wizard_running,
            flows::list_flows,
            flows::load_flow,
            flows::save_flow,
            flows::delete_flow,
            flows::get_flow_execution,
            flows::set_node_output,
            flows::get_node_output,
            flows::init_flow_execution,
            flows::update_node_status,
            flows::append_flow_log,
            flows::finish_flow_execution,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
