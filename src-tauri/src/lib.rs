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
            commands::looks::theme_samples,
            commands::looks::make_thumbnail,
            commands::looks::style_preview,
            commands::looks::apply_look,
            commands::login::get_login,
            commands::login::apply_login,
            commands::panel::list_panels,
            commands::panel::set_panel,
            commands::panel::add_panel,
            commands::panel::remove_panel,
            commands::kwin::list_desktops,
            commands::kwin::add_desktop,
            commands::kwin::remove_desktop,
            commands::kwin::rename_desktop,
            commands::kwin::set_desktop_rows,
            commands::kwin::set_desktop_wrap,
            commands::kwin::list_kwin_effects,
            commands::kwin::list_kwin_scripts,
            commands::kwin::list_window_switchers,
            commands::kwin::set_kwin_effect,
            commands::kwin::set_kwin_script,
            commands::kwin::set_window_switcher,
            commands::install::install_options,
            commands::install::get_new,
            commands::install::install_file,
            commands::behavior::list_behavior,
            commands::behavior::set_behavior,
            commands::rules::list_rules,
            commands::rules::save_rule,
            commands::rules::remove_rule,
            commands::rules::set_rule_enabled,
            commands::install::removable_looks,
            commands::install::remove_look,
            commands::panel::list_widget_types,
            commands::panel::list_panel_widgets,
            commands::panel::add_panel_widget,
            commands::panel::remove_panel_widget,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soplos Look Manager");
}
