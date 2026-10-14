pub mod commands;
pub mod domain;
pub mod error;
pub mod infra;
pub mod services;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;
    tauri::Builder::default()
        .enable_macos_default_menu(false)
        .manage(state::AppState::default())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_icon(tauri::include_image!("icons/32x32.png"))?;
            }
            services::desktop::setup(app)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let state = handle.state::<state::AppState>();
                    let Ok(_guard) = state.operations.try_lock() else {
                        continue;
                    };
                    // The access state carries actionable failures to the shell without logging credentials.
                    if services::workspace::sync_primary(&handle).await.is_ok() {
                        let _ = services::workspace::sync_active(&handle).await;
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    services::desktop::hide_main(window.app_handle());
                }
                tauri::WindowEvent::Focused(false)
                | tauri::WindowEvent::Moved(_)
                | tauri::WindowEvent::Resized(_) => {
                    services::desktop::hide_hint(window.app_handle())
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_info,
            commands::workspace::workspace_init,
            commands::workspace::save_settings,
            commands::workspace::open_primary,
            commands::workspace::sync_sessions,
            commands::workspace::search_users,
            commands::workspace::refresh_directory,
            commands::workspace::drag_window,
            commands::workspace::add_account,
            commands::workspace::check_primary_access,
            commands::workspace::relogin_primary,
            commands::workspace::relogin_session,
            commands::workspace::select_session,
            commands::workspace::browser_bounds,
            commands::workspace::close_session,
            commands::workspace::close_workspace,
            commands::workspace::browser_action,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if matches!(event, tauri::RunEvent::Reopen { .. }) {
                services::desktop::show_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
