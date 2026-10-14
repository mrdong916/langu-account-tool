use serde::ser::{Serialize, Serializer};
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("主账号正在浏览外部页面，请返回系统首页后重试")]
    AuthUnavailable,
    #[error("登录已过期，请重新登录主账号")]
    Unauthorized,
    #[error("暂无权限")]
    Forbidden,
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Tauri(#[from] tauri::Error),
    #[error("本地文件操作失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("数据格式错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("本地用户缓存操作失败: {0}")]
    Database(#[from] rusqlite::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
