use crate::{
    domain::{
        access::PrimaryAccess,
        workspace::{Bounds, Session, Settings},
    },
    error::{AppError, AppResult},
};
use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};

#[derive(Default)]
pub struct Workspace {
    pub access: PrimaryAccess,
    pub settings: Settings,
    pub sessions: BTreeMap<String, Session>,
    pub active: Option<String>,
    pub visible: bool,
    pub bounds: Bounds,
    pub pending_hint: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub workspace: Mutex<Workspace>,
    pub operations: tokio::sync::Mutex<()>,
    pub directory: Mutex<crate::services::directory::DirectoryState>,
    pub hint_generation: std::sync::atomic::AtomicU64,
    pub hint_ready: std::sync::atomic::AtomicBool,
}

impl AppState {
    pub fn lock(&self) -> AppResult<MutexGuard<'_, Workspace>> {
        self.workspace
            .lock()
            .map_err(|_| AppError::Message("会话状态不可用，请重启客户端".into()))
    }
}
