use crate::{
    domain::{
        directory::{score, DirectoryStatus},
        workspace::{Account, Settings, UserPage},
    },
    error::{AppError, AppResult},
    infra::{
        directory as storage,
        langu::{self, Credential},
    },
    state::AppState,
};
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    time::Instant,
};
use tauri::{AppHandle, Manager};

const DAY: i64 = 86_400_000;

#[derive(Default)]
pub struct DirectoryState {
    scope: String,
    key: String,
    generation: u64,
    users: BTreeMap<String, Account>,
    pub status: DirectoryStatus,
    attempted_at: Option<Instant>,
}

pub fn reset(app: &AppHandle) {
    if let Ok(mut state) = app.state::<AppState>().directory.lock() {
        let generation = state.generation + 1;
        *state = DirectoryState {
            generation,
            ..Default::default()
        };
    }
}

fn path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .app_local_data_dir()?
        .join("directory/users.sqlite3"))
}

pub fn enrich(app: &AppHandle, account: &mut Account) {
    if let Ok(directory) = app.state::<AppState>().directory.lock() {
        if let Some(cached) = directory
            .users
            .get(&account.id)
            .filter(|user| user.username == account.username)
        {
            account.department = cached.department.clone();
            account.department_code = cached.department_code.clone();
            account.work_no = cached.work_no.clone();
            account.status = cached.status.clone();
            if account.realname.is_empty() {
                account.realname = cached.realname.clone();
            }
        }
    }
    crate::domain::directory::index(account);
}

pub fn search(app: &AppHandle, keyword: &str, page: u32) -> AppResult<UserPage> {
    if keyword.len() > 400 {
        return Err(AppError::Message("检索内容过长".into()));
    }
    if !app.state::<AppState>().lock()?.access.ready() {
        return Err(AppError::Forbidden);
    }
    let state = app.state::<AppState>();
    let directory = state
        .directory
        .lock()
        .map_err(|_| AppError::Message("用户目录不可用".into()))?;
    let mut matches: Vec<_> = directory
        .users
        .values()
        .filter(|user| user.status == "1")
        .filter_map(|user| score(user, keyword).map(|rank| (rank, user)))
        .collect();
    matches.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.username.cmp(&b.1.username))
            .then_with(|| a.1.id.cmp(&b.1.id))
    });
    let page = page.max(1);
    Ok(UserPage {
        total: matches.len() as u64,
        records: matches
            .into_iter()
            .skip((page as usize - 1).saturating_mul(20))
            .take(20)
            .map(|(_, user)| user.clone())
            .collect(),
        page,
        page_size: 20,
    })
}

pub fn schedule(
    app: &AppHandle,
    config: &Settings,
    credential: &Credential,
    owner: &str,
    force_full: bool,
) -> AppResult<()> {
    let scope = format!(
        "{:x}",
        md5::compute(serde_json::to_vec(&(
            config.login_url.as_str(),
            config.api_base_url.as_str(),
            owner
        ))?)
    );
    let key = credential.key();
    let generation;
    {
        let state = app.state::<AppState>();
        let mut directory = state
            .directory
            .lock()
            .map_err(|_| AppError::Message("用户目录不可用".into()))?;
        if directory.scope != scope || directory.key != key {
            *directory = DirectoryState {
                scope: scope.clone(),
                key,
                generation: directory.generation + 1,
                ..Default::default()
            };
        }
        let due = directory
            .status
            .last_full_sync
            .is_none_or(|t| chrono::Utc::now().timestamp_millis() - t >= DAY);
        let retry_due = !directory.status.error.is_empty()
            && directory
                .attempted_at
                .is_some_and(|t| t.elapsed().as_secs() >= 60);
        if directory.status.syncing
            || (!force_full && directory.attempted_at.is_some() && !retry_due && !due)
        {
            return Ok(());
        }
        // Do not spin on a failed first sync or a failed daily reconciliation.
        if !force_full
            && directory
                .attempted_at
                .is_some_and(|t| t.elapsed().as_secs() < 60)
        {
            return Ok(());
        }
        directory.generation += 1;
        generation = directory.generation;
        directory.status.syncing = true;
        directory.status.fetched = 0;
        directory.status.total = 0;
        directory.status.error.clear();
        directory.attempted_at = Some(Instant::now());
    }
    super::workspace::publish(app);
    let app = app.clone();
    let config = config.clone();
    let credential = credential.clone();
    tauri::async_runtime::spawn(async move {
        let result = synchronize(&app, &config, &credential, &scope, generation, force_full).await;
        let current = app
            .state::<AppState>()
            .directory
            .lock()
            .is_ok_and(|d| d.generation == generation);
        if !current {
            return;
        }
        if let Err(ref error) = result {
            if matches!(error, AppError::Unauthorized | AppError::Forbidden) {
                let state = app.state::<AppState>();
                let _guard = state.operations.lock().await;
                if valid(&app, generation, &credential).await.is_ok() {
                    let _ = super::workspace::handle_primary_error(&app, error).await;
                }
            }
        }
        if let Ok(mut directory) = app.state::<AppState>().directory.lock() {
            if directory.generation == generation {
                directory.status.syncing = false;
                if let Err(error) = result {
                    directory.status.error = error.to_string();
                }
            }
        }
        super::workspace::publish(&app);
    });
    Ok(())
}

async fn valid(app: &AppHandle, generation: u64, credential: &Credential) -> AppResult<()> {
    let access = {
        let state = app.state::<AppState>();
        let ws = state.lock()?;
        ws.access.ready() && ws.access.key.as_deref() == Some(credential.key().as_str())
    };
    if !access
        || !app
            .state::<AppState>()
            .directory
            .lock()
            .is_ok_and(|d| d.generation == generation)
    {
        return Err(AppError::Message("主账号已变化，同步已取消".into()));
    }
    if super::workspace::credential(app).await.ok().as_ref() != Some(credential) {
        return Err(AppError::Message("主账号已变化，同步已取消".into()));
    }
    Ok(())
}

async fn synchronize(
    app: &AppHandle,
    config: &Settings,
    credential: &Credential,
    scope: &str,
    generation: u64,
    force_full: bool,
) -> AppResult<()> {
    let db = path(app)?;
    let disk_path = db.clone();
    let disk_scope = scope.to_string();
    let cache =
        tauri::async_runtime::spawn_blocking(move || storage::load(&disk_path, &disk_scope))
            .await
            .map_err(|_| AppError::Message("读取用户缓存失败".into()))??;
    valid(app, generation, credential).await?;
    let since = cache.last_sync;
    let last_full = cache.last_full_sync;
    {
        let state = app.state::<AppState>();
        let mut directory = state
            .directory
            .lock()
            .map_err(|_| AppError::Message("用户目录不可用".into()))?;
        if directory.generation != generation {
            return Ok(());
        }
        directory.users = cache
            .users
            .into_iter()
            .map(|user| (user.id.clone(), user))
            .collect();
        directory.status.count = directory.users.len();
        directory.status.last_sync = since;
        directory.status.last_full_sync = last_full;
    }
    super::workspace::publish(app);
    let until = langu::server_millis(config).await?;
    let full = force_full
        || since.is_none()
        || since.is_some_and(|t| t > until)
        || last_full.is_none_or(|t| until - t >= DAY);
    let fields = if full {
        vec![None]
    } else {
        vec![Some("createTime"), Some("updateTime")]
    };
    let mut updates = BTreeMap::new();
    let mut fetched_total = 0;
    for field in fields {
        let mut page = 1;
        let mut received = 0;
        let mut seen_ids = HashSet::new();
        let mut expected_total = None;
        loop {
            valid(app, generation, credential).await?;
            let range = field.map(|f| (f, since.unwrap_or(0).saturating_sub(120_000), until));
            let response = langu::directory_page(config, credential, page, range).await?;
            valid(app, generation, credential).await?;
            let duplicate = response
                .records
                .iter()
                .any(|user| !seen_ids.insert(user.id.clone()));
            if (response.records.is_empty() && received < response.total)
                || duplicate
                || expected_total.is_some_and(|total| total != response.total)
                || received + response.records.len() as u64 > response.total
            {
                return Err(AppError::Message(
                    "用户分页数据不完整，已保留原缓存，请重试".into(),
                ));
            }
            expected_total = Some(response.total);
            received += response.records.len() as u64;
            for user in response.records {
                updates.insert(user.id.clone(), user);
            }
            {
                let state = app.state::<AppState>();
                let mut directory = state
                    .directory
                    .lock()
                    .map_err(|_| AppError::Message("用户目录不可用".into()))?;
                if directory.generation != generation {
                    return Ok(());
                }
                directory.status.fetched = updates.len();
                directory.status.total = fetched_total + response.total;
            }
            super::workspace::publish(app);
            if received >= response.total {
                fetched_total += response.total;
                break;
            }
            page += 1;
        }
    }
    let active_ids = if full {
        updates
            .values()
            .filter(|user| user.status == "1")
            .map(|user| user.id.clone())
            .collect()
    } else {
        let mut ids = HashSet::new();
        let mut page = 1;
        let mut expected_total = None;
        loop {
            valid(app, generation, credential).await?;
            let (batch, total) = langu::active_directory_ids(config, credential, page).await?;
            if expected_total.is_some_and(|expected| expected != total)
                || (batch.is_empty() && (ids.len() as u64) < total)
                || batch.into_iter().any(|id| !ids.insert(id))
                || ids.len() as u64 > total
            {
                return Err(AppError::Message(
                    "正常账号列表已变化，已保留原缓存，请重试".into(),
                ));
            }
            expected_total = Some(total);
            if ids.len() as u64 == total {
                break;
            }
            page += 1;
        }
        ids
    };
    valid(app, generation, credential).await?;
    let handle = app.clone();
    let scope = scope.to_string();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        let state = handle.state::<AppState>();
        let mut directory = state
            .directory
            .lock()
            .map_err(|_| AppError::Message("用户目录不可用".into()))?;
        if directory.generation != generation {
            return Ok(());
        }
        let mut users = if full {
            BTreeMap::new()
        } else {
            directory.users.clone()
        };
        users.extend(updates);
        users.retain(|id, user| user.status == "1" && active_ids.contains(id));
        storage::save_snapshot(
            &db,
            &scope,
            &users.values().cloned().collect::<Vec<_>>(),
            full,
            until,
        )?;
        directory.users = users;
        directory.status.count = directory.users.len();
        directory.status.last_sync = Some(until);
        if full {
            directory.status.last_full_sync = Some(until);
        }
        Ok(())
    })
    .await
    .map_err(|_| AppError::Message("保存用户缓存失败".into()))??;
    Ok(())
}
