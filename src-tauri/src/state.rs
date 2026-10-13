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
}

#[derive(Default)]
pub struct AppState {
    pub workspace: Mutex<Workspace>,
    pub operations: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn lock(&self) -> AppResult<MutexGuard<'_, Workspace>> {
        self.workspace
            .lock()
            .map_err(|_| AppError::Message("会话状态不可用，请重启客户端".into()))
    }
}
