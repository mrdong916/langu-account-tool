use crate::{
    domain::workspace::{http_url, Bounds, Settings, UserPage},
    error::{AppError, AppResult},
    infra::{langu, settings},
    services::workspace as service,
    state::AppState,
};
use serde_json::Value;
use tauri::{AppHandle, Manager, State, Webview};

#[tauri::command]
pub async fn workspace_init(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    if state.lock()?.sessions.is_empty() {
        state.lock()?.settings = settings::load(&app)?;
    }
    service::snapshot(&app)
}

#[tauri::command]
pub async fn save_settings(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    mut config: Settings,
) -> AppResult<Settings> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    if !state.lock()?.sessions.is_empty() {
        return Err(AppError::Message("请先关闭当前工作区再修改系统设置".into()));
    }
    config.validate()?;
    let previous = state.lock()?.settings.clone();
    if previous.login_url != config.login_url || previous.api_base_url != config.api_base_url {
        config.username.clear();
    }
    settings::save(&app, &config)?;
    state.lock()?.settings = config.clone();
    service::publish(&app);
    Ok(config)
}

#[tauri::command]
pub async fn open_primary(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    if state.lock()?.sessions.contains_key("primary") {
        return service::select(&app, "primary".into());
    }
    let config = state.lock()?.settings.clone();
    service::create(&app, "primary".into(), "primary", &config, None)
}

#[tauri::command]
pub async fn sync_sessions(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Value> {
    service::trusted(&caller)?;
    let Ok(_guard) = state.operations.try_lock() else {
        return service::snapshot(&app);
    };
    service::sync_primary(&app).await?;
    service::sync_active(&app).await?;
    service::snapshot(&app)
}

#[tauri::command]
pub async fn search_users(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    keyword: String,
    page: u32,
) -> AppResult<UserPage> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    if keyword.len() > 200 {
        return Err(AppError::Message("检索内容过长".into()));
    }
    let config = state.lock()?.settings.clone();
    let credential = service::authorized_primary(&app).await?;
    let result = langu::search(&config, &credential, keyword, page).await;
    service::ensure_primary(&app, &credential).await?;
    if let Err(error) = &result {
        service::handle_primary_error(&app, error).await?;
    }
    result
}

#[tauri::command]
pub async fn check_primary_access(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    service::authorized_primary(&app).await?;
    Ok(())
}

#[tauri::command]
pub async fn relogin_primary(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    service::return_to_login(&app).await
}

#[tauri::command]
pub async fn relogin_session(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    let session = state
        .lock()?
        .sessions
        .get(&id)
        .filter(|s| s.role == "secondary")
        .cloned()
        .ok_or_else(|| AppError::Message("子账号会话不存在".into()))?;
    let username = session
        .account
        .as_ref()
        .ok_or_else(|| AppError::Message("子账号信息缺失".into()))?
        .username
        .clone();
    let credential = service::authorized_primary(&app).await?;
    let config = state.lock()?.settings.clone();
    let result = langu::impersonate(&config, &credential, username).await;
    service::ensure_primary(&app, &credential).await?;
    let payload = match result {
        Ok(payload) => payload,
        Err(error) => {
            service::handle_primary_error(&app, &error).await?;
            return Err(error);
        }
    };
    let view = app
        .get_webview(&id)
        .ok_or_else(|| AppError::Message("子账号页面已关闭".into()))?;
    *session
        .bootstrap
        .lock()
        .map_err(|_| AppError::Message("登录状态不可用".into()))? = Some(payload);
    service::update_session(&app, &id, |s| {
        s.status = "checking".into();
        s.validated_at = None;
        s.auth_key = None;
    });
    let landing = http_url(&config.login_url)?
        .join(&config.landing_path)
        .map_err(|_| AppError::Message("跳转地址无效".into()))?;
    if let Err(error) = view.navigate(landing) {
        if let Ok(mut payload) = session.bootstrap.lock() {
            payload.take();
        }
        service::update_session(&app, &id, |s| s.status = "error".into());
        return Err(error.into());
    }
    service::select(&app, id)
}

#[tauri::command]
pub async fn add_account(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    username: String,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    let credential = service::authorized_primary(&app).await?;
    if username.is_empty() || username.len() > 300 {
        return Err(AppError::Message("账号名称无效".into()));
    }
    let existing = state
        .lock()?
        .sessions
        .values()
        .find(|s| s.account.as_ref().is_some_and(|a| a.username == username))
        .map(|s| s.id.clone());
    if let Some(id) = existing {
        return service::select(&app, id);
    }
    let config = state.lock()?.settings.clone();
    let result = langu::impersonate(&config, &credential, username).await;
    service::ensure_primary(&app, &credential).await?;
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            service::handle_primary_error(&app, &error).await?;
            return Err(error);
        }
    };
    let id = format!("account-{}", uuid::Uuid::new_v4());
    service::create(&app, id, "secondary", &config, Some(result))
}

#[tauri::command]
pub async fn select_session(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    service::sync_primary(&app).await?;
    service::select(&app, id.clone())?;
    service::update_session(&app, &id, |s| s.validated_at = None);
    service::sync_active(&app).await
}

#[tauri::command]
pub async fn browser_bounds(
    caller: Webview,
    app: AppHandle,
    bounds: Bounds,
    visible: bool,
) -> AppResult<()> {
    service::trusted(&caller)?;
    service::set_bounds(&app, bounds, visible)
}

#[tauri::command]
pub async fn close_session(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    if id == "primary" {
        return Err(AppError::Message("请使用关闭工作区来关闭主账号".into()));
    }
    service::close(&app, &id)
}

#[tauri::command]
pub async fn close_workspace(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    clear: bool,
) -> AppResult<()> {
    service::trusted(&caller)?;
    let _guard = state.operations.lock().await;
    service::close_children(&app)?;
    if clear {
        if let Some(view) = app.get_webview("primary") {
            view.clear_all_browsing_data()?;
        }
        let config = {
            let mut ws = state.lock()?;
            ws.settings.username.clear();
            ws.settings.clone()
        };
        settings::save(&app, &config)?;
    }
    service::close(&app, "primary")
}

#[tauri::command]
pub async fn browser_action(
    caller: Webview,
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
    value: Option<String>,
) -> AppResult<()> {
    service::trusted(&caller)?;
    if !state.lock()?.sessions.contains_key(&id) {
        return Err(AppError::Message("账号会话不存在".into()));
    }
    {
        let ws = state.lock()?;
        if !ws.access.ready()
            && (id != "primary"
                || matches!(
                    ws.access.status,
                    crate::domain::access::AccessStatus::Denied
                        | crate::domain::access::AccessStatus::Checking
                        | crate::domain::access::AccessStatus::Unavailable
                ))
        {
            return Err(AppError::Message("请先登录有权限的主账号".into()));
        }
    }
    let view = app
        .get_webview(&id)
        .ok_or_else(|| AppError::Message("浏览页面已关闭".into()))?;
    match action.as_str() {
        "back" => view.eval("history.back()")?,
        "forward" => view.eval("history.forward()")?,
        "reload" => view.reload()?,
        "stop" => view.eval("window.stop()")?,
        "home" => {
            let config = state.lock()?.settings.clone();
            view.navigate(http_url(&config.login_url)?)?;
        }
        "navigate" => view.navigate(http_url(value.as_deref().unwrap_or_default())?)?,
        "devtools" => {
            if view.is_devtools_open() {
                view.close_devtools();
            } else {
                view.open_devtools();
            }
        }
        "zoom" => {
            let zoom: f64 = value
                .as_deref()
                .unwrap_or("1")
                .parse()
                .map_err(|_| AppError::Message("缩放比例无效".into()))?;
            if !zoom.is_finite() || !(0.5..=2.0).contains(&zoom) {
                return Err(AppError::Message("缩放范围为 50% 至 200%".into()));
            }
            view.set_zoom(zoom)?;
            service::update_session(&app, &id, |s| s.zoom = zoom);
        }
        _ => return Err(AppError::Message("不支持的浏览器操作".into())),
    }
    Ok(())
}
