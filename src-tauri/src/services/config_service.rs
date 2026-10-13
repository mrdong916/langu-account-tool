use crate::domain::app::AppInfo;
use crate::error::AppResult;

#[derive(Default)]
pub struct ConfigService;

impl ConfigService {
    pub async fn get_app_info(&self) -> AppResult<AppInfo> {
        Ok(AppInfo {
            name: "兰谷多账号工具".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            runtime: "Tauri".to_owned(),
        })
    }
}
