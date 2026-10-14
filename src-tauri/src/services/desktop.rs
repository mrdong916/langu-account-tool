use crate::{error::AppResult, state::AppState};
use std::sync::atomic::Ordering;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn hide_hint(app: &AppHandle) {
    app.state::<AppState>()
        .hint_generation
        .fetch_add(1, Ordering::SeqCst);
    if let Some(hint) = app.get_webview_window("switch-hint") {
        let _ = hint.hide();
    }
}

pub fn hide_main(app: &AppHandle) {
    hide_hint(app);
    if let Some(window) = app.get_window("main") {
        let _ = window.hide();
    }
}

fn action(app: &AppHandle, id: &str) {
    match id {
        "show" => show_main(app),
        "hide" => hide_main(app),
        "quit" => app.exit(0),
        "search" | "settings" | "refresh-directory" | "primary" | "previous" | "next"
        | "close-workspace" => {
            show_main(app);
            let _ = app.emit_to(tauri::EventTarget::webview("main"), "desktop-action", id);
        }
        _ => {}
    }
}

pub fn setup(app: &mut tauri::App) -> AppResult<()> {
    let settings = MenuItemBuilder::with_id("settings", "系统设置…")
        .accelerator("CmdOrCtrl+Comma")
        .build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "隐藏窗口")
        .accelerator("CmdOrCtrl+W")
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出兰谷多账号工具")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let search = MenuItemBuilder::with_id("search", "搜索账号…")
        .accelerator("CmdOrCtrl+Shift+K")
        .build(app)?;
    let application = SubmenuBuilder::new(app, "兰谷多账号工具")
        .text("show", "打开主窗口")
        .item(&settings)
        .separator()
        .item(&hide)
        .item(&quit)
        .build()?;
    let accounts = SubmenuBuilder::new(app, "账号")
        .item(&search)
        .text("primary", "返回主账号")
        .text("previous", "上一个账号")
        .text("next", "下一个账号")
        .separator()
        .text("refresh-directory", "刷新用户缓存")
        .text("close-workspace", "关闭工作区…")
        .build()?;
    let edit = SubmenuBuilder::new(app, "编辑")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    app.set_menu(
        MenuBuilder::new(app)
            .items(&[&application, &accounts, &edit])
            .build()?,
    )?;
    app.on_menu_event(|app, event| action(app, event.id().as_ref()));
    let tray_menu = MenuBuilder::new(app)
        .text("show", "打开兰谷多账号工具")
        .text("search", "搜索账号…")
        .text("refresh-directory", "刷新用户缓存")
        .text("settings", "系统设置…")
        .separator()
        .text("quit", "退出")
        .build()?;
    TrayIconBuilder::with_id("main-tray")
        .tooltip("兰谷多账号工具")
        .icon(tauri::include_image!("icons/32x32.png"))
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    if let Some(parent) = app.get_webview_window("main") {
        let handle = app.handle().clone();
        let hint =
            WebviewWindowBuilder::new(app, "switch-hint", WebviewUrl::App("switch.html".into()))
                .title("账号切换提示")
                .inner_size(560.0, 200.0)
                .visible(false)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .focused(false)
                .focusable(false)
                .skip_taskbar(true)
                .parent(&parent)?
                .on_navigation(|url| {
                    url.path() == "/switch.html"
                        && ((url.scheme() == "tauri" && url.host_str() == Some("localhost"))
                            || url.origin().ascii_serialization() == "http://tauri.localhost"
                            || url.origin().ascii_serialization() == "http://127.0.0.1:1430")
                })
                .on_page_load(move |_, event| {
                    if event.event() == tauri::webview::PageLoadEvent::Finished {
                        handle
                            .state::<AppState>()
                            .hint_ready
                            .store(true, Ordering::SeqCst);
                    }
                })
                .build()?;
        hint.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

pub fn show_hint(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.hint_ready.load(Ordering::SeqCst) {
        return;
    }
    let Some(parent) = app.get_window("main") else {
        return;
    };
    if !parent.is_visible().unwrap_or(false) || parent.is_minimized().unwrap_or(true) {
        return;
    }
    let payload = {
        let Ok(mut ws) = state.lock() else {
            return;
        };
        if !ws.visible || !ws.access.ready() {
            return;
        }
        let Some(id) = ws.pending_hint.clone() else {
            return;
        };
        if ws.active.as_ref() != Some(&id) {
            ws.pending_hint = None;
            return;
        }
        let Some(session) = ws.sessions.get(&id) else {
            return;
        };
        let Some(mut account) = session.account.clone() else {
            return;
        };
        let primary = session.role == "primary";
        super::directory::enrich(app, &mut account);
        ws.pending_hint = None;
        (account, primary, ws.bounds)
    };
    let Some(hint) = app.get_webview_window("switch-hint") else {
        return;
    };
    let Ok(position) = parent.inner_position() else {
        return;
    };
    let Ok(scale) = parent.scale_factor() else {
        return;
    };
    let position = position.to_logical::<f64>(scale);
    let width = payload.2.width.clamp(280.0, 620.0);
    let _ = hint.set_size(tauri::LogicalSize::new(width, 200.0));
    let _ = hint.set_position(tauri::LogicalPosition::new(
        position.x + payload.2.x + (payload.2.width - width) / 2.0,
        position.y + payload.2.y + (payload.2.height - 200.0).max(0.0) / 2.0,
    ));
    let Ok(data) = serde_json::to_string(
        &serde_json::json!({"name": if payload.0.realname.is_empty() { &payload.0.username } else { &payload.0.realname }, "detail": format!("{} · {}{}", payload.0.username, payload.0.department, if payload.1 { " · 主账号" } else { "" })}),
    ) else {
        return;
    };
    let generation = state.hint_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let callback_app = app.clone();
    let _ = hint.eval_with_callback(
        format!("window.prepareAccountHint?.({data}); true"),
        move |_| {
            if callback_app
                .state::<AppState>()
                .hint_generation
                .load(Ordering::SeqCst)
                == generation
                && callback_app.get_window("main").is_some_and(|main| {
                    main.is_visible().unwrap_or(false)
                        && (cfg!(target_os = "windows") || main.is_focused().unwrap_or(false))
                })
            {
                if let Some(hint) = callback_app.get_webview_window("switch-hint") {
                    if hint.show().is_err() {
                        return;
                    }
                    let handle = callback_app.clone();
                    if hint
                        .eval_with_callback("window.playAccountHint?.(); true", move |_| {
                            let handle = handle.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(1350)).await;
                                if handle
                                    .state::<AppState>()
                                    .hint_generation
                                    .load(Ordering::SeqCst)
                                    == generation
                                {
                                    if let Some(hint) = handle.get_webview_window("switch-hint") {
                                        let _ = hint.hide();
                                    }
                                }
                            });
                        })
                        .is_err()
                    {
                        hide_hint(&callback_app);
                    }
                }
            }
        },
    );
}
