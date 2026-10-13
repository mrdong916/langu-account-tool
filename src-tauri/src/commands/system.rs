use crate::domain::app::AppInfo;
use crate::error::AppResult;

#[tauri::command]
pub async fn get_app_info(caller: tauri::Webview) -> AppResult<AppInfo> {
    crate::services::workspace::trusted(&caller)?;
    crate::services::config_service::ConfigService
        .get_app_info()
        .await
}
