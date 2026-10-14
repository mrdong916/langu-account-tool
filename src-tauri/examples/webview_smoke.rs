//! Run with the local test system; see docs/verification.md. Never uses saved accounts.
use langu_account_tool_lib::{
    domain::workspace::{Bounds, Settings},
    error::{AppError, AppResult},
    infra::langu,
    services::{desktop, directory, workspace},
    state::AppState,
};
use std::time::Duration;
use tauri::{AppHandle, Manager, Webview};

async fn wait_directory(app: &AppHandle, count: usize) -> AppResult<()> {
    for _ in 0..100 {
        let status = app
            .state::<AppState>()
            .directory
            .lock()
            .unwrap()
            .status
            .clone();
        if !status.syncing && status.last_sync.is_some() {
            if status.count != count || !status.error.is_empty() {
                return Err(AppError::Message(format!(
                    "Directory count/error mismatch: {}/{count}: {}",
                    status.count, status.error
                )));
            }
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(AppError::Message(
        "Directory synchronization timed out".into(),
    ))
}

async fn fixture(path: &str, body: serde_json::Value) -> AppResult<()> {
    reqwest::Client::new()
        .post(format!("http://127.0.0.1:1432/fixture/{path}"))
        .json(&body)
        .send()
        .await
        .map_err(|_| AppError::Message("Fixture request failed".into()))?
        .error_for_status()
        .map_err(|_| AppError::Message("Fixture response failed".into()))?;
    Ok(())
}

async fn check_directory(app: &AppHandle) -> AppResult<()> {
    wait_directory(app, 245).await?;
    for query in ["zhangsan", "zs", "财务 张", "a1000"] {
        let result = directory::search(app, query, 1)?;
        if result.total != 1 || result.records[0].department != "财务部" {
            return Err(AppError::Message(format!("Local search failed: {query}")));
        }
    }
    let first = app
        .state::<AppState>()
        .directory
        .lock()
        .unwrap()
        .status
        .clone();
    let now = chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    fixture(
        "users",
        serde_json::json!({"id":"fixture-1", "realDepart_dictText":"研发部", "updateTime":now}),
    )
    .await?;
    fixture("users", serde_json::json!({"id":"fixture-new", "username":"test.new", "realname":"李四", "realDepart_dictText":"研发部", "status":1, "createTime":now})).await?;
    directory::reset(app); // Simulate a new process loading the persisted owner partition.
    workspace::sync_primary(app).await?;
    wait_directory(app, 246).await?;
    if directory::search(app, "研发 张", 1)?.total != 1
        || directory::search(app, "lisi", 1)?.total != 1
    {
        return Err(AppError::Message(
            "Incremental create/update query failed".into(),
        ));
    }
    if app
        .state::<AppState>()
        .directory
        .lock()
        .unwrap()
        .status
        .last_full_sync
        != first.last_full_sync
    {
        return Err(AppError::Message(
            "Restart should sync incrementally".into(),
        ));
    }
    // A normal-only delta cannot return newly frozen users; the ID reconciliation must prune them.
    fixture(
        "users",
        serde_json::json!({"id":"fixture-2", "status":2, "updateTime":now}),
    )
    .await?;
    directory::reset(app);
    workspace::sync_primary(app).await?;
    wait_directory(app, 245).await?;
    if directory::search(app, "test.user.02", 1)?.total != 0
        || directory::search(app, "test.frozen", 1)?.total != 0
        || directory::search(app, "test.resigned", 1)?.total != 0
        || directory::search(app, "test.unknown", 1)?.total != 0
    {
        return Err(AppError::Message(
            "Inactive accounts leaked into the directory".into(),
        ));
    }
    fixture(
        "users",
        serde_json::json!({"id":"fixture-2", "status":1, "updateTime":now}),
    )
    .await?;
    directory::reset(app);
    workspace::sync_primary(app).await?;
    wait_directory(app, 246).await?;
    fixture(
        "users",
        serde_json::json!({"id":"fixture-new", "remove":true}),
    )
    .await?;
    let before_failure = app
        .state::<AppState>()
        .directory
        .lock()
        .unwrap()
        .status
        .last_sync;
    fixture("directory", serde_json::json!({"failPage":2})).await?;
    langu_account_tool_lib::commands::workspace::refresh_directory(
        app.get_webview("main").unwrap(),
        app.clone(),
        app.state(),
    )
    .await?;
    for _ in 0..100 {
        if !app
            .state::<AppState>()
            .directory
            .lock()
            .unwrap()
            .status
            .syncing
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let failed = app
        .state::<AppState>()
        .directory
        .lock()
        .unwrap()
        .status
        .clone();
    if failed.error.is_empty() || failed.last_sync != before_failure || failed.count != 246 {
        return Err(AppError::Message(
            "Failed sync replaced cached snapshot or cursor".into(),
        ));
    }
    fixture("directory", serde_json::json!({"failPage":0})).await?;
    langu_account_tool_lib::commands::workspace::refresh_directory(
        app.get_webview("main").unwrap(),
        app.clone(),
        app.state(),
    )
    .await?;
    wait_directory(app, 245).await?;
    if directory::search(app, "test.new", 1)?.total != 0 {
        return Err(AppError::Message(
            "Full sync did not prune deleted user".into(),
        ));
    }
    Ok(())
}

async fn expect_script(view: &Webview, script: &str) -> AppResult<()> {
    for _ in 0..10 {
        if workspace::eval_result(view, script.into())
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(AppError::Message(format!(
        "WebView smoke assertion failed in {}: {script}",
        view.label()
    )))
}

#[cfg(target_os = "macos")]
async fn check_bounds(app: &AppHandle, view: &Webview) -> AppResult<()> {
    let window = app.get_window("main").unwrap();
    for (width, height, x) in [(1360.0, 900.0, 260.0), (960.0, 700.0, 68.0)] {
        window.set_size(tauri::LogicalSize::new(width, height))?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        workspace::set_bounds(
            app,
            Bounds {
                x,
                y: 57.0,
                width: width - x,
                height: height - 140.0,
            },
            true,
        )?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        view.with_webview(move |native| unsafe {
            let view = &*native.inner().cast::<objc2_web_kit::WKWebView>();
            let parent = view.superview().unwrap();
            let frame = view.frame();
            let top = parent.frame().size.height - frame.origin.y - frame.size.height;
            let _ = tx.send((top - parent.safeAreaInsets().top, frame.origin.x));
        })?;
        let (top, left) = rx
            .await
            .map_err(|_| AppError::Message("Bounds callback lost".into()))?;
        if (top - 57.0).abs() > 1.0 || (left - x).abs() > 1.0 {
            return Err(AppError::Message(format!(
                "Toolbar overlap: top={top}, left={left}"
            )));
        }
    }
    Ok(())
}

async fn check(app: &AppHandle) -> AppResult<()> {
    if app.config().identifier != "com.langu.accounttool.smoke" {
        return Err(AppError::Message(
            "Use the isolated smoke app identifier".into(),
        ));
    }
    let mode = std::env::var("LANGU_SMOKE_MODE").unwrap_or_else(|_| "local".into());
    let _ = std::fs::remove_file(
        app.path()
            .app_local_data_dir()?
            .join("directory/users.sqlite3"),
    );
    if mode == "local" {
        fixture("users", serde_json::json!({"id":"fixture-2", "status":1})).await?;
        fixture("users", serde_json::json!({"id":"fixture-1", "realDepart_dictText":"财务部", "updateTime":"2026-01-01 00:00:00"})).await?;
        fixture(
            "users",
            serde_json::json!({"id":"fixture-new", "remove":true}),
        )
        .await?;
        fixture("directory", serde_json::json!({"failPage":0})).await?;
    }
    let config = Settings {
        login_url: match mode.as_str() {
            "local" => "http://127.0.0.1:1432/",
            "blocked" => "http://ats-blocked.invalid/", // Reserved domain; macOS ATS denies it.
            "remote" => "http://languwin.com/",
            _ => return Err(AppError::Message("Unknown smoke mode".into())),
        }
        .into(),
        api_base_url: "http://127.0.0.1:1432".into(),
        remember: false,
        landing_path: "/app".into(),
        ..Settings::default()
    };
    app.state::<AppState>().lock()?.settings = config.clone();
    workspace::set_bounds(
        app,
        Bounds {
            x: 260.0,
            y: 0.0,
            width: 1100.0,
            height: 850.0,
        },
        true,
    )?;
    workspace::create(app, "primary".into(), "primary", &config, None)?;
    let primary = app.get_webview("primary").unwrap();
    if mode == "blocked" {
        tokio::time::sleep(Duration::from_secs(2)).await;
        // This used to panic in Wry's native URL getter and terminate the whole process.
        if matches!(workspace::read_auth(app, "primary").await, Ok(Some(_))) {
            return Err(AppError::Message(
                "Blocked navigation returned credentials".into(),
            ));
        }
        workspace::close(app, "primary")?;
        return Ok(());
    }
    if mode == "remote" {
        return expect_script(&primary, "location.hostname === 'languwin.com' && document.readyState === 'complete' && document.body.innerText.length > 0").await;
    }
    expect_script(
        &primary,
        "document.getElementById('account')?.textContent === 'Current account: test.admin'",
    )
    .await?;
    #[cfg(target_os = "macos")]
    check_bounds(app, &primary).await?;
    workspace::sync_primary(app).await?;
    check_directory(app).await?;
    let credential = workspace::authorized_primary(app).await?;
    let payload = langu::impersonate(&config, &credential, "test.user.01".into()).await?;
    workspace::create(
        app,
        "smoke-child".into(),
        "secondary",
        &config,
        Some(payload),
    )?;
    let child = app.get_webview("smoke-child").unwrap();
    expect_script(
        &child,
        "document.getElementById('account')?.textContent === 'Current account: test.user.01'",
    )
    .await?;
    expect_script(
        &primary,
        "localStorage.setItem('draft', 'PRIMARY-DRAFT'); true",
    )
    .await?;
    expect_script(&child, "localStorage.setItem('draft', 'CHILD-DRAFT'); true").await?;
    workspace::select(app, "primary".into())?;
    expect_script(
        &primary,
        "localStorage.getItem('draft') === 'PRIMARY-DRAFT'",
    )
    .await?;
    desktop::show_main(app);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let was_focused = app.get_window("main").unwrap().is_focused()?;
    workspace::select(app, "smoke-child".into())?;
    workspace::sync_active(app).await?;
    if app.state::<AppState>().lock()?.sessions["smoke-child"].status != "ready" {
        return Err(AppError::Message("Child login not ready".into()));
    }
    expect_script(&child, "localStorage.getItem('draft') === 'CHILD-DRAFT'").await?;
    // Existing-session selection must not queue behind background HTTP work.
    {
        let state = app.state::<AppState>();
        let _guard = state.operations.lock().await;
        for id in ["primary", "smoke-child"] {
            tokio::time::timeout(
                Duration::from_secs(1),
                langu_account_tool_lib::commands::workspace::select_session(
                    app.get_webview("main").unwrap(),
                    app.clone(),
                    id.into(),
                ),
            )
            .await
            .map_err(|_| {
                AppError::Message("Account switching waited for background operations".into())
            })??;
        }
    }
    if app.state::<AppState>().lock()?.sessions["smoke-child"]
        .validated_at
        .is_some()
    {
        return Err(AppError::Message(
            "Account switching did not schedule a child recheck".into(),
        ));
    }
    let hint = app
        .get_webview("switch-hint")
        .ok_or_else(|| AppError::Message("Switch hint missing".into()))?;
    expect_script(&hint, "typeof window.showAccountHint === 'function'").await?;
    expect_script(
        &hint,
        "document.getElementById('name')?.textContent === '张三'",
    )
    .await?;
    if was_focused && !app.get_window("main").unwrap().is_focused()? {
        return Err(AppError::Message("Hint stole keyboard focus".into()));
    }
    desktop::hide_main(app);
    if app.get_window("main").unwrap().is_visible()? {
        return Err(AppError::Message("Window did not hide".into()));
    }
    desktop::show_main(app);
    expect_script(&child, "localStorage.getItem('draft') === 'CHILD-DRAFT'").await?;
    let main_auth = workspace::read_auth(app, "primary").await?;
    let child_auth = workspace::read_auth(app, "smoke-child").await?;
    if main_auth
        .as_ref()
        .and_then(|v| v["user"]["username"].as_str())
        != Some("test.admin")
        || child_auth
            .as_ref()
            .and_then(|v| v["user"]["username"].as_str())
            != Some("test.user.01")
    {
        return Err(AppError::Message("Account isolation check failed".into()));
    }
    // Logout and reload must not replay the consumed bootstrap payload.
    child.eval("document.getElementById('logout').click()")?;
    expect_script(
        &child,
        "!localStorage.getItem('pro__Access-Token') && document.readyState === 'complete'",
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    workspace::sync_active(app).await?;
    if app.state::<AppState>().lock()?.sessions["smoke-child"].status != "expired" {
        return Err(AppError::Message("Logout replayed credentials".into()));
    }
    langu_account_tool_lib::commands::workspace::relogin_session(
        app.get_webview("main").unwrap(),
        app.clone(),
        app.state(),
        "smoke-child".into(),
    )
    .await?;
    expect_script(
        &child,
        "document.getElementById('account')?.textContent === 'Current account: test.user.01'",
    )
    .await?;
    workspace::sync_active(app).await?;
    expect_script(&child, "localStorage.getItem('draft') === 'CHILD-DRAFT'").await?;
    expect_script(&primary, "localStorage.getItem('draft') === 'PRIMARY-DRAFT' && document.getElementById('account')?.textContent === 'Current account: test.admin'").await?;
    workspace::select(app, "primary".into())?;
    primary.navigate("http://localhost:1432/second".parse().unwrap())?;
    expect_script(
        &primary,
        "location.origin === 'http://localhost:1432' && document.readyState === 'complete'",
    )
    .await?;
    workspace::sync_primary(app).await?;
    if app.state::<AppState>().lock()?.access.status
        != langu_account_tool_lib::domain::access::AccessStatus::Away
    {
        return Err(AppError::Message(
            "External navigation was treated as expired login".into(),
        ));
    }
    if workspace::select(app, "smoke-child".into()).is_ok() {
        return Err(AppError::Message(
            "External page could authorize account management".into(),
        ));
    }
    primary.navigate(config.login_url.parse().unwrap())?;
    expect_script(&primary, "location.origin === 'http://127.0.0.1:1432' && document.getElementById('account')?.textContent === 'Current account: test.admin'").await?;
    workspace::sync_primary(app).await?;
    if !app.state::<AppState>().lock()?.access.ready() {
        return Err(AppError::Message(
            "Returning from an external page did not restore access".into(),
        ));
    }
    expect_script(
        &primary,
        "localStorage.getItem('draft') === 'PRIMARY-DRAFT'",
    )
    .await?;
    expect_script(&child, "localStorage.getItem('draft') === 'CHILD-DRAFT'").await?;
    fixture("permission", serde_json::json!({"allowed":false})).await?;
    if workspace::authorized_primary(app).await.is_ok()
        || workspace::select(app, "smoke-child".into()).is_ok()
    {
        return Err(AppError::Message(
            "Fast switching bypassed a rejected permission check".into(),
        ));
    }
    fixture("permission", serde_json::json!({"allowed":true})).await?;
    workspace::authorized_primary(app).await?;
    workspace::close_children(app)?;
    workspace::close(app, "primary")
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            desktop::setup(app)?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result = check(&handle).await;
                match &result {
                    Ok(()) => eprintln!("WebView smoke passed"),
                    Err(error) => eprintln!("WebView smoke failed: {error}"),
                }
                handle.exit(if result.is_ok() { 0 } else { 1 });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("smoke app failed");
}
