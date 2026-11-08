mod commands;
mod system_theme;

// What pkexec runs as the administrator; main() calls it before Tauri starts.
pub use commands::login::run_ops_file;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin registered: it needs to intercept and
        // exit a second launch before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            system_theme::get_system_theme,
            commands::environment::get_environment_info,
            commands::looks::list_looks,
            commands::looks::read_preview,
            commands::looks::apply_look,
            commands::login::get_login,
            commands::login::apply_login,
            commands::panel::list_panels,
            commands::panel::set_panel,
            commands::panel::add_panel,
            commands::panel::remove_panel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soplos Look Manager");
}
