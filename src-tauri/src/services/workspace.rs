use crate::{
    domain::{
        access::{AccessStatus, HEARTBEAT_INTERVAL},
        workspace::{http_url, Bounds, Session, Settings},
    },
    error::{AppError, AppResult},
    infra::{
        langu::{self, Credential},
        settings,
    },
    state::AppState,
};
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder},
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Webview, WebviewUrl,
};

pub fn trusted(caller: &Webview) -> AppResult<()> {
    if caller.label() != "main" {
        return Err(AppError::Message("此页面无权操作客户端".into()));
    }
    Ok(())
}

pub fn snapshot(app: &AppHandle) -> AppResult<Value> {
    let state = app.state::<AppState>();
    let ws = state.lock()?;
    let mut sessions = ws.sessions.values().collect::<Vec<_>>();
    sessions.sort_by_key(|session| session.created_at);
    Ok(
        json!({"settings": ws.settings, "sessions": sessions, "active": ws.active, "access": ws.access}),
    )
}

pub fn publish(app: &AppHandle) {
    if let Ok(snapshot) = snapshot(app) {
        let _ = app.emit_to(
            tauri::EventTarget::webview("main"),
            "workspace-changed",
            snapshot,
        );
    }
}

pub fn update_session(app: &AppHandle, id: &str, change: impl FnOnce(&mut Session)) {
    if let Ok(mut ws) = app.state::<AppState>().lock() {
        if let Some(session) = ws.sessions.get_mut(id) {
            change(session);
        }
    }
    publish(app);
}

pub fn position(app: &AppHandle) -> AppResult<()> {
    let (active, visible, bounds, ids, access) = {
        let state = app.state::<AppState>();
        let ws = state.lock()?;
        (
            ws.active.clone(),
            ws.visible,
            ws.bounds,
            ws.sessions.keys().cloned().collect::<Vec<_>>(),
            ws.access.status,
        )
    };
    for id in ids {
        if let Some(view) = app.get_webview(&id) {
            if visible
                && (access == AccessStatus::Ready
                    || (id == "primary"
                        && matches!(access, AccessStatus::Pending | AccessStatus::Expired)))
                && active.as_deref() == Some(&id)
                && bounds.width > 0.0
                && bounds.height > 0.0
            {
                #[cfg(target_os = "macos")]
                crate::infra::macos::set_bounds(&view, bounds)?;
                #[cfg(not(target_os = "macos"))]
                view.set_bounds(tauri::Rect {
                    position: LogicalPosition::new(bounds.x, bounds.y).into(),
                    size: LogicalSize::new(bounds.width, bounds.height).into(),
                })?;
                view.show()?;
            } else {
                view.hide()?;
            }
        }
    }
    Ok(())
}

pub fn select(app: &AppHandle, id: String) -> AppResult<()> {
    {
        let state = app.state::<AppState>();
        let mut ws = state.lock()?;
        if !ws.sessions.contains_key(&id) {
            return Err(AppError::Message("账号会话不存在".into()));
        }
        if id != "primary" && !ws.access.ready() {
            return Err(AppError::Message("请先登录有权限的主账号".into()));
        }
        ws.active = Some(id);
    }
    position(app)?;
    publish(app);
    Ok(())
}

pub fn create(
    app: &AppHandle,
    id: String,
    role: &str,
    config: &Settings,
    payload: Option<Value>,
) -> AppResult<()> {
    let login = http_url(&config.login_url)?;
    let origin = login.origin().ascii_serialization();
    let url = if role == "primary" {
        login
    } else {
        login
            .join(&config.landing_path)
            .map_err(|_| AppError::Message("跳转地址无效".into()))?
    };
    let account = payload
        .as_ref()
        .map(|v| langu::account(&v["result"]["userInfo"], config))
        .transpose()?;
    let has_bootstrap = payload.is_some();
    let mut initial_url = url.clone();
    if cfg!(target_os = "macos") && has_bootstrap {
        initial_url = tauri::Url::parse("about:blank").expect("valid blank URL");
    }
    let mut builder = WebviewBuilder::new(&id, WebviewUrl::External(initial_url)).devtools(true);
    // Each account has a distinct engine profile, including cookies and IndexedDB.
    let profile = if role == "primary" && config.remember {
        format!("main-{:x}", md5::compute(&config.login_url))
    } else {
        id.clone()
    };
    #[cfg(not(target_os = "macos"))]
    {
        builder = builder.data_directory(
            app.path()
                .app_local_data_dir()?
                .join("profiles")
                .join(&profile),
        );
    }
    #[cfg(target_os = "macos")]
    {
        builder = builder.data_store_identifier(md5::compute(&profile).0);
    }
    if role != "primary" || !config.remember {
        builder = builder.incognito(true);
    }
    // Consume the bootstrap payload once; refresh or logout must never replay it.
    let bootstrap = std::sync::Arc::new(std::sync::Mutex::new(payload));
    let pending_bootstrap = bootstrap.clone();
    let handle = app.clone();
    let label = id.clone();
    builder = builder.on_navigation(move |url| {
        if cfg!(target_os = "macos") && url.as_str() == "about:blank" {
            return true;
        }
        if !matches!(url.scheme(), "http" | "https") {
            return false;
        }
        #[cfg(target_os = "macos")]
        if url.path() == crate::infra::macos::BOOTSTRAP_PATH {
            return true;
        }
        update_session(&handle, &label, |s| s.url = url.to_string());
        true
    });
    let handle = app.clone();
    let label = id.clone();
    let landing = url.clone();
    builder = builder
        .on_page_load(move |view, event| {
            #[cfg(target_os = "macos")]
            let internal_page = event.url().path() == crate::infra::macos::BOOTSTRAP_PATH
                || event.url().as_str() == "about:blank";
            #[cfg(not(target_os = "macos"))]
            let internal_page = false;
            update_session(&handle, &label, |s| {
                s.loading = event.event() == PageLoadEvent::Started;
                if !internal_page {
                    s.url = event.url().to_string();
                }
            });
            let can_bootstrap = event.event() == PageLoadEvent::Finished
                && event.url().origin().ascii_serialization() == origin;
            #[cfg(target_os = "macos")]
            let can_bootstrap =
                can_bootstrap && event.url().path() == crate::infra::macos::BOOTSTRAP_PATH;
            if can_bootstrap {
                let payload = pending_bootstrap
                    .lock()
                    .ok()
                    .and_then(|mut value| value.take());
                if let Some(payload) = payload {
                    update_session(view.app_handle(), view.label(), |s| s.loading = true);
                    let script = format!(
                        "{}\nbootstrapLanguAccount({}, {});",
                        include_str!("../../scripts/bootstrap.js"),
                        serde_json::to_string(&origin).unwrap(),
                        serde_json::to_string(&payload).unwrap()
                    );
                    let target = landing.clone();
                    let callback_view = view.clone();
                    let result = view.eval_with_callback(script, move |result| {
                        if result != "true" || callback_view.navigate(target.clone()).is_err() {
                            update_session(
                                callback_view.app_handle(),
                                callback_view.label(),
                                |s| {
                                    s.status = "error".into();
                                    s.loading = false;
                                },
                            );
                        }
                    });
                    if result.is_err() {
                        update_session(view.app_handle(), view.label(), |s| {
                            s.status = "error".into();
                            s.loading = false;
                        });
                    }
                }
            }
        })
        .on_document_title_changed(|view, title| {
            update_session(view.app_handle(), view.label(), |s| s.title = title);
        });
    let handle = app.clone();
    let label = id.clone();
    builder = builder.on_new_window(move |url, _| {
        if matches!(url.scheme(), "http" | "https") {
            if let Some(view) = handle.get_webview(&label) {
                let _ = view.navigate(url);
            }
        }
        NewWindowResponse::Deny
    });
    let session = Session {
        id: id.clone(),
        created_at: chrono::Utc::now().timestamp_micros(),
        role: role.into(),
        account,
        url: url.to_string(),
        title: "正在打开".into(),
        loading: true,
        status: if role == "primary" {
            "pending"
        } else {
            "checking"
        }
        .into(),
        zoom: 1.0,
        validated_at: None,
        auth_key: None,
        bootstrap,
    };
    app.state::<AppState>()
        .lock()?
        .sessions
        .insert(id.clone(), session);
    let window = app
        .get_window("main")
        .ok_or_else(|| AppError::Message("主窗口不可用".into()))?;
    match window.add_child(
        builder,
        LogicalPosition::new(-10000.0, -10000.0),
        LogicalSize::new(800.0, 600.0),
    ) {
        Ok(view) => {
            view.hide()?;
            #[cfg(target_os = "macos")]
            if has_bootstrap {
                prepare_login(&view, url)?;
            }
        }
        Err(error) => {
            app.state::<AppState>().lock()?.sessions.remove(&id);
            return Err(error.into());
        }
    }
    select(app, id)
}

pub fn prepare_login(view: &Webview, landing: tauri::Url) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    crate::infra::macos::prepare_login(view, landing)?;
    #[cfg(not(target_os = "macos"))]
    view.navigate(landing)?;
    Ok(())
}

pub async fn read_auth(app: &AppHandle, id: &str) -> AppResult<Option<Value>> {
    let config = app.state::<AppState>().lock()?.settings.clone();
    let Some(view) = app.get_webview(id) else {
        return Ok(None);
    };
    let expected = http_url(&config.login_url)?.origin().ascii_serialization();
    // Validate origin and the top-level frame inside the script, at the time of the read.
    let script = format!(
        "{}\nreadLanguAuth({});",
        include_str!("../../scripts/read-auth.js"),
        serde_json::to_string(&expected)?
    );
    let value = eval_value(&view, script).await?;
    Ok(if value.is_object() { Some(value) } else { None })
}

pub async fn credential(app: &AppHandle) -> AppResult<Credential> {
    let auth = read_auth(app, "primary")
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(Credential::from_auth(&auth))
}

pub async fn sync_primary(app: &AppHandle) -> AppResult<()> {
    check_primary(app, false).await
}

fn set_access(app: &AppHandle, status: AccessStatus, message: &str) -> AppResult<()> {
    {
        let state = app.state::<AppState>();
        let mut ws = state.lock()?;
        ws.access.status = status;
        ws.access.message = message.into();
        if status != AccessStatus::Ready {
            ws.active = ws
                .sessions
                .contains_key("primary")
                .then(|| "primary".into());
        }
        if let Some(primary) = ws.sessions.get_mut("primary") {
            primary.status = match status {
                AccessStatus::Ready => "ready",
                AccessStatus::Expired => "expired",
                AccessStatus::Denied => "denied",
                AccessStatus::Checking => "checking",
                AccessStatus::Unavailable => "error",
                AccessStatus::Pending => "pending",
            }
            .into();
        }
    }
    position(app)?;
    publish(app);
    Ok(())
}

pub async fn return_to_login(app: &AppHandle) -> AppResult<()> {
    let config = app.state::<AppState>().lock()?.settings.clone();
    if let Some(view) = app.get_webview("primary") {
        let origin = http_url(&config.login_url)?.origin().ascii_serialization();
        let script = format!(
            "{}\nclearLanguAuth({});",
            include_str!("../../scripts/clear-auth.js"),
            serde_json::to_string(&origin)?
        );
        eval_result(&view, script).await?;
        view.navigate(http_url(&config.login_url)?)?;
    }
    set_access(app, AccessStatus::Expired, "主账号登录已过期，请重新登录")
}

pub async fn handle_primary_error(app: &AppHandle, error: &AppError) -> AppResult<()> {
    match error {
        AppError::Unauthorized => {
            set_access(app, AccessStatus::Expired, "主账号登录已过期，请重新登录")?;
            return_to_login(app).await?;
        }
        AppError::Forbidden => set_access(app, AccessStatus::Denied, "暂无权限")?,
        _ => {}
    }
    Ok(())
}

async fn check_primary(app: &AppHandle, force: bool) -> AppResult<()> {
    let config = app.state::<AppState>().lock()?.settings.clone();
    if !app
        .state::<AppState>()
        .lock()?
        .sessions
        .contains_key("primary")
    {
        return Ok(());
    }
    let auth = read_auth(app, "primary").await?;
    let Some(auth) = auth else {
        let previous = app.state::<AppState>().lock()?.access.status;
        if previous != AccessStatus::Pending && previous != AccessStatus::Expired {
            handle_primary_error(app, &AppError::Unauthorized).await?;
        }
        return Ok(());
    };
    let account = langu::account(&auth["user"], &config)?;
    let credential = Credential::from_auth(&auth);
    let key = credential.key();
    let owner = format!("{}:{}:{}", credential.tenant, account.id, account.username);
    let (changed_owner, changed_key, needs_check, heartbeat_due) = {
        let state = app.state::<AppState>();
        let ws = state.lock()?;
        (
            ws.access.owner.as_ref().is_some_and(|old| old != &owner),
            ws.access.key.as_ref() != Some(&key),
            ws.access.needs_check(&key, force),
            ws.access.heartbeat_due(),
        )
    };
    if changed_owner {
        close_children(app)?;
    }
    if !needs_check {
        // Foreground operations may refresh permission checks, but never postpone keepalive.
        if heartbeat_due && app.state::<AppState>().lock()?.access.ready() {
            app.state::<AppState>().lock()?.access.heartbeat_at = Some(std::time::Instant::now());
            if let Err(error) = langu::heartbeat(&config, &credential).await {
                if self::credential(app).await.ok().as_ref() == Some(&credential) {
                    handle_primary_error(app, &error).await?;
                }
                return Err(error);
            }
        }
        return Ok(());
    }
    {
        let state = app.state::<AppState>();
        let mut ws = state.lock()?;
        ws.access.key = Some(key);
        ws.access.owner = Some(owner);
        ws.access.checked_at = Some(std::time::Instant::now());
    }
    // Do not hide a previously authorized view while performing a routine recheck.
    if !app.state::<AppState>().lock()?.access.ready() || changed_key {
        set_access(app, AccessStatus::Checking, "正在校验主账号权限")?;
    }
    update_session(app, "primary", |s| {
        s.account = Some(account.clone());
    });
    let result = langu::check_permission(&config, &credential).await;
    if self::credential(app).await.ok().as_ref() != Some(&credential) {
        set_access(
            app,
            AccessStatus::Pending,
            "主账号登录状态已变化，等待重新校验",
        )?;
        app.state::<AppState>().lock()?.access.key = None;
        return Err(AppError::Message("主账号登录状态已变化，请重试".into()));
    }
    if let Err(error) = result {
        match error {
            AppError::Unauthorized | AppError::Forbidden => {
                handle_primary_error(app, &error).await?
            }
            _ => set_access(
                app,
                AccessStatus::Unavailable,
                "权限校验失败，请检查网络后重试",
            )?,
        }
        return Err(error);
    }
    set_access(app, AccessStatus::Ready, "")?;
    if config.username != account.username {
        let updated = {
            let state = app.state::<AppState>();
            let mut ws = state.lock()?;
            ws.settings.username = account.username;
            ws.settings.clone()
        };
        settings::save(app, &updated)?;
    }
    if heartbeat_due {
        app.state::<AppState>().lock()?.access.heartbeat_at = Some(std::time::Instant::now());
        if let Err(error) = langu::heartbeat(&config, &credential).await {
            if self::credential(app).await.ok().as_ref() == Some(&credential) {
                handle_primary_error(app, &error).await?;
            }
            return Err(error);
        }
    }
    Ok(())
}

pub async fn authorized_primary(app: &AppHandle) -> AppResult<Credential> {
    check_primary(app, true).await?;
    if !app.state::<AppState>().lock()?.access.ready() {
        select(app, "primary".into())?;
        return Err(AppError::Message("请先登录有权限的主账号".into()));
    }
    let credential = credential(app).await?;
    ensure_primary(app, &credential).await?;
    Ok(credential)
}

pub async fn ensure_primary(app: &AppHandle, expected: &Credential) -> AppResult<()> {
    let current = credential(app).await.ok();
    if current.is_none() {
        handle_primary_error(app, &AppError::Unauthorized).await?;
        return Err(AppError::Unauthorized);
    }
    if current.as_ref() != Some(expected) {
        set_access(
            app,
            AccessStatus::Pending,
            "主账号登录状态已变化，请重新校验",
        )?;
        app.state::<AppState>().lock()?.access.key = None;
        return Err(AppError::Unauthorized);
    }
    let valid = {
        let state = app.state::<AppState>();
        let ws = state.lock()?;
        ws.access.ready() && ws.access.key.as_deref() == Some(expected.key().as_str())
    };
    if !valid {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn eval_result(view: &Webview, script: String) -> AppResult<bool> {
    Ok(eval_value(view, script).await? == Value::Bool(true))
}

async fn eval_value(view: &Webview, script: String) -> AppResult<Value> {
    // Wry's macOS URL getter panics when a first navigation fails and WKWebView.URL is nil.
    // Script evaluation can fail or time out safely, without reading that native URL.
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    view.eval_with_callback(script, move |result| {
        if let Some(tx) = tx.lock().ok().and_then(|mut tx| tx.take()) {
            let _ = tx.send(result);
        }
    })?;
    let result = tokio::time::timeout(Duration::from_secs(4), rx)
        .await
        .map_err(|_| AppError::Message("页面尚未就绪，请稍后重试".into()))?
        .map_err(|_| AppError::Message("页面已关闭".into()))?;
    Ok(serde_json::from_str(&result)?)
}

pub async fn sync_active(app: &AppHandle) -> AppResult<()> {
    let (active, config) = {
        let state = app.state::<AppState>();
        let ws = state.lock()?;
        if !ws.access.ready() {
            return Ok(());
        }
        (ws.active.clone(), ws.settings.clone())
    };
    let Some(id) = active else {
        return Ok(());
    };
    let Some(view) = app.get_webview(&id) else {
        return Ok(());
    };
    let location = eval_value(&view, "window.location.href".into()).await?;
    let Some(location) = location.as_str() else {
        return Ok(());
    };
    let url = http_url(location)?;
    #[cfg(target_os = "macos")]
    if url.path() == crate::infra::macos::BOOTSTRAP_PATH {
        return Ok(());
    }
    update_session(app, &id, |session| session.url = url.to_string());
    if id != "primary" && url.origin() == http_url(&config.login_url)?.origin() {
        let auth = read_auth(app, &id).await?;
        let current = auth
            .as_ref()
            .map(|value| langu::account(&value["user"], &config))
            .transpose()?;
        let Some(auth) = auth else {
            let loading = app
                .state::<AppState>()
                .lock()?
                .sessions
                .get(&id)
                .is_some_and(|s| s.loading);
            if !loading {
                update_session(app, &id, |session| session.status = "expired".into());
            }
            return Ok(());
        };
        let credential = Credential::from_auth(&auth);
        let key = credential.key();
        let validate = {
            let state = app.state::<AppState>();
            let ws = state.lock()?;
            ws.sessions.get(&id).is_some_and(|s| {
                s.auth_key.as_ref() != Some(&key)
                    || (s.status != "expired"
                        && s.validated_at
                            .is_none_or(|t| t.elapsed() >= HEARTBEAT_INTERVAL))
            })
        };
        update_session(app, &id, |session| {
            if let Some(current) = current {
                session.account = Some(current);
            }
        });
        if validate {
            let result = langu::heartbeat(&config, &credential).await;
            if read_auth(app, &id)
                .await?
                .as_ref()
                .map(Credential::from_auth)
                .as_ref()
                != Some(&credential)
            {
                return Ok(());
            }
            update_session(app, &id, |session| {
                session.auth_key = Some(key);
                session.validated_at = Some(std::time::Instant::now());
                match result {
                    Ok(()) => session.status = "ready".into(),
                    Err(AppError::Unauthorized) => session.status = "expired".into(),
                    Err(_) => session.status = "error".into(),
                }
            });
        }
    }
    Ok(())
}

pub fn close(app: &AppHandle, id: &str) -> AppResult<()> {
    if let Some(view) = app.get_webview(id) {
        view.close()?;
    }
    let state = app.state::<AppState>();
    let mut ws = state.lock()?;
    ws.sessions.remove(id);
    if id == "primary" {
        ws.access = Default::default();
    }
    if ws.active.as_deref() == Some(id) {
        ws.active = ws
            .sessions
            .contains_key("primary")
            .then(|| "primary".into());
    }
    drop(ws);
    position(app)?;
    publish(app);
    Ok(())
}

pub fn close_children(app: &AppHandle) -> AppResult<()> {
    let ids = app
        .state::<AppState>()
        .lock()?
        .sessions
        .keys()
        .filter(|id| id.as_str() != "primary")
        .cloned()
        .collect::<Vec<_>>();
    for id in ids {
        close(app, &id)?;
    }
    Ok(())
}

pub fn set_bounds(app: &AppHandle, bounds: Bounds, visible: bool) -> AppResult<()> {
    if [bounds.x, bounds.y, bounds.width, bounds.height]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0 || *v > 50000.0)
    {
        return Err(AppError::Message("浏览区域尺寸无效".into()));
    }
    {
        let state = app.state::<AppState>();
        let mut ws = state.lock()?;
        ws.bounds = bounds;
        ws.visible = visible;
    }
    position(app)
}
