use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use tauri::Url;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub login_url: String,
    pub api_base_url: String,
    pub landing_path: String,
    pub remember: bool,
    pub username: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            login_url: "http://languwin.com/".into(),
            api_base_url: "http://api.languwin.com".into(),
            landing_path: "/".into(),
            remember: true,
            username: String::new(),
        }
    }
}

pub fn http_url(input: &str) -> AppResult<Url> {
    let url = Url::parse(input.trim())
        .map_err(|_| AppError::Message("请输入完整的 http:// 或 https:// 地址".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::Message(
            "地址仅支持 HTTP(S)，且不能包含用户名或密码".into(),
        ));
    }
    Ok(url)
}

impl Settings {
    pub fn validate(&mut self) -> AppResult<()> {
        self.login_url = http_url(&self.login_url)?.to_string();
        let api = http_url(&self.api_base_url)?;
        if api.query().is_some() || api.fragment().is_some() {
            return Err(AppError::Message("接口地址不能包含查询参数或片段".into()));
        }
        self.api_base_url = api.to_string().trim_end_matches('/').into();
        let login = http_url(&self.login_url)?;
        let landing = login
            .join(&self.landing_path)
            .map_err(|_| AppError::Message("登录后地址无效".into()))?;
        if login.origin() != landing.origin() {
            return Err(AppError::Message("登录后的跳转地址必须与登录页同源".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub realname: String,
    #[serde(default)]
    pub department: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub created_at: i64,
    pub role: String,
    pub account: Option<Account>,
    pub url: String,
    pub title: String,
    pub loading: bool,
    pub status: String,
    pub zoom: f64,
    #[serde(skip)]
    pub validated_at: Option<std::time::Instant>,
    #[serde(skip)]
    pub auth_key: Option<String>,
    #[serde(skip)]
    pub bootstrap: std::sync::Arc<std::sync::Mutex<Option<serde_json::Value>>>,
}

#[derive(Clone, Copy, Default, Deserialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPage {
    pub records: Vec<Account>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_privileged_urls_and_cross_origin_landing() {
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "http://u:p@example.com",
        ] {
            assert!(http_url(url).is_err());
        }
        let mut settings = Settings {
            landing_path: "//other.example/".into(),
            ..Settings::default()
        };
        assert!(settings.validate().is_err());
    }
}
