//! Run with the local test system; see docs/verification.md. Never uses saved accounts.
use langu_account_tool_lib::{
    domain::workspace::{Bounds, Settings},
    error::{AppError, AppResult},
    infra::langu,
    services::workspace,
    state::AppState,
};
use std::time::Duration;
use tauri::{AppHandle, Manager, Webview};

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
    Err(AppError::Message("WebView smoke assertion failed".into()))
}

async fn check(app: &AppHandle) -> AppResult<()> {
    if app.config().identifier != "com.langu.accounttool.smoke" {
        return Err(AppError::Message(
            "Use the isolated smoke app identifier".into(),
        ));
    }
    let mode = std::env::var("LANGU_SMOKE_MODE").unwrap_or_else(|_| "local".into());
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
    workspace::sync_primary(app).await?;
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
    workspace::select(app, "smoke-child".into())?;
    workspace::sync_active(app).await?;
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
    workspace::close_children(app)?;
    workspace::close(app, "primary")
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
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
