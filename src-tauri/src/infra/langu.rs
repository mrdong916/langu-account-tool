use crate::{
    domain::workspace::{Account, Settings, UserPage},
    error::{AppError, AppResult},
};
use chrono::{FixedOffset, Utc};
use reqwest::{redirect::Policy, Client};
use serde_json::{json, Value};
use std::{collections::BTreeMap, time::Duration};

// Protocol constant from langu-web-oa/src/utils/encryption/signMd5Utils.js.
const SIGNATURE_SUFFIX: &str = "dd05f1c54d63749eda95f9fa6d49v442a";

#[derive(Clone, PartialEq)]
pub struct Credential {
    pub token: String,
    pub tenant: String,
}

impl Credential {
    pub fn from_auth(auth: &Value) -> Self {
        Self {
            token: auth["token"].as_str().unwrap_or_default().into(),
            tenant: auth["tenant"].as_str().unwrap_or("0").into(),
        }
    }

    pub fn key(&self) -> String {
        format!(
            "{:x}",
            md5::compute(format!("{}:{}", self.tenant, self.token))
        )
    }
}

fn response_error(status: u16, body: &Value) -> Option<AppError> {
    let code = body["code"]
        .as_i64()
        .or_else(|| body["code"].as_str()?.parse().ok());
    let message = body["message"].as_str().unwrap_or_default().to_lowercase();
    let expired = [
        "token失效",
        "token已失效",
        "token过期",
        "token已过期",
        "登录已过期",
        "登录失效",
        "登录超时",
        "token expired",
        "invalid token",
    ]
    .iter()
    .any(|text| message.contains(text));
    if status == 401 || code == Some(401) || expired {
        Some(AppError::Unauthorized)
    } else if status == 403 || code == Some(403) {
        Some(AppError::Forbidden)
    } else {
        None
    }
}

pub fn has_internal_login_permission(result: &Value) -> bool {
    // allAuth is the system-wide catalog, NOT this user's grants.
    result["auth"].as_array().is_some_and(|auth| {
        auth.iter().any(|item| {
            item["action"] == "system:internalLoginByAdmin"
                && item["type"] != "2"
                && item["type"] != 2
        })
    })
}

pub async fn check_permission(settings: &Settings, credential: &Credential) -> AppResult<()> {
    let result = request(
        settings,
        credential,
        "/sys/permission/getUserPermissionByToken",
        BTreeMap::new(),
        false,
    )
    .await?;
    if !has_internal_login_permission(&result) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn heartbeat(settings: &Settings, credential: &Credential) -> AppResult<()> {
    request(
        settings,
        credential,
        "/sys/sysConfig/getByCode",
        BTreeMap::from([("code".into(), "SYSTEM_NOTIFICATIONS".into())]),
        false,
    )
    .await?;
    Ok(())
}

pub fn signature(params: &BTreeMap<String, String>) -> String {
    let body = serde_json::to_string(params).expect("string map serialization");
    format!("{:X}", md5::compute(format!("{body}{SIGNATURE_SUFFIX}")))
}

pub fn account(value: &Value, settings: &Settings) -> AppResult<Account> {
    let username = value["username"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::Message("用户数据缺少 username".into()))?;
    let avatar = value["avatar"].as_str().unwrap_or_default();
    let avatar = if avatar.is_empty() {
        String::new()
    } else if avatar.starts_with("https://") || avatar.starts_with("http://") {
        avatar.into()
    } else {
        format!(
            "{}/sys/common/static/{}",
            settings.api_base_url,
            avatar.trim_start_matches('/')
        )
    };
    Ok(Account {
        id: value["id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| username.into()),
        username: username.into(),
        avatar,
        realname: value["realname"].as_str().unwrap_or_default().into(),
        department: value["orgCodeTxt"].as_str().unwrap_or_default().into(),
    })
}

async fn request(
    settings: &Settings,
    credential: &Credential,
    path: &str,
    params: BTreeMap<String, String>,
    post: bool,
) -> AppResult<Value> {
    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(Policy::none())
        .build()
        .map_err(|_| AppError::Message("无法初始化接口客户端".into()))?;
    let started = std::time::Instant::now();
    let server_time = client
        .get(format!(
            "{}/sys/common/getServerTimestamp",
            settings.api_base_url
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await;
    let millis = match server_time {
        Ok(response) => response
            .json::<Value>()
            .await
            .ok()
            .filter(|v| v["success"] == true)
            .and_then(|v| v["result"].as_i64()),
        Err(_) => None,
    };
    let time = millis
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|t| t + chrono::Duration::milliseconds(started.elapsed().as_millis() as i64 / 2))
        .unwrap_or_else(Utc::now);
    let timestamp = time
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y%m%d%H%M%S")
        .to_string();
    let url = format!("{}{path}", settings.api_base_url);
    let mut builder = if post {
        client.post(url).json(&params)
    } else {
        // The site's Axios interceptor adds _t AFTER getSign().
        client
            .get(url)
            .query(&params)
            .query(&[("_t", time.timestamp().to_string())])
    };
    builder = builder
        .header("X-Access-Token", &credential.token)
        .header("tenant-id", &credential.tenant)
        .header("X-Sign", signature(&params))
        .header("X-TIMESTAMP", timestamp)
        .header(
            "Origin",
            tauri::Url::parse(&settings.login_url)
                .unwrap()
                .origin()
                .ascii_serialization(),
        );
    let response = builder
        .send()
        .await
        .map_err(|_| AppError::Message("接口连接失败或超时，请检查网络和接口地址".into()))?;
    let status = response.status();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    if let Some(error) = response_error(status.as_u16(), &body) {
        return Err(error);
    }
    if !status.is_success() {
        return Err(AppError::Message(format!(
            "系统接口返回 HTTP {}",
            status.as_u16()
        )));
    }
    if !body.is_object() {
        return Err(AppError::Message("接口返回了无法识别的数据".into()));
    }
    if body["success"] != true {
        let message = body["message"].as_str().unwrap_or("接口操作失败");
        let message = message.replace(&credential.token, "[redacted]");
        return Err(AppError::Message(message.chars().take(240).collect()));
    }
    Ok(body["result"].clone())
}

pub async fn search(
    settings: &Settings,
    credential: &Credential,
    keyword: String,
    page: u32,
) -> AppResult<UserPage> {
    let mut params = BTreeMap::from([
        ("column".into(), "createTime".into()),
        ("order".into(), "desc".into()),
        ("pageNo".into(), page.max(1).to_string()),
        ("pageSize".into(), "20".into()),
    ]);
    if !keyword.trim().is_empty() {
        params.insert("username".into(), format!("*{}*", keyword.trim()));
    }
    let result = request(settings, credential, "/sys/user/list", params, false).await?;
    let records = result["records"]
        .as_array()
        .ok_or_else(|| AppError::Message("用户列表缺少 records".into()))?
        .iter()
        .map(|v| account(v, settings))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(UserPage {
        records,
        total: result["total"].as_u64().unwrap_or(0),
        page: page.max(1),
        page_size: 20,
    })
}

pub async fn impersonate(
    settings: &Settings,
    credential: &Credential,
    username: String,
) -> AppResult<Value> {
    let result = request(
        settings,
        credential,
        "/sys/internalLoginByAdmin",
        BTreeMap::from([("username".into(), username.clone())]),
        true,
    )
    .await?;
    if result["token"].as_str().filter(|v| !v.is_empty()).is_none() {
        return Err(AppError::Message("代登录接口未返回 token".into()));
    }
    if result["userInfo"]["username"].as_str() != Some(&username) {
        return Err(AppError::Message(
            "代登录接口返回的账号与选择不一致，已取消打开".into(),
        ));
    }
    Ok(json!({"result": result, "tenant": credential.tenant}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mock_api(
        status: u16,
        body: Value,
        endpoint: &'static str,
    ) -> (Settings, std::thread::JoinHandle<()>) {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            for step in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    request.push_str(&line);
                }
                let (status, value) = if step == 0 {
                    assert!(request.starts_with("GET /sys/common/getServerTimestamp"));
                    (
                        200,
                        json!({"success": true, "result": Utc::now().timestamp_millis()}),
                    )
                } else {
                    assert!(request.starts_with(&format!("GET {endpoint}")));
                    assert!(request
                        .to_lowercase()
                        .contains("x-access-token: fixture-test-only"));
                    (status, body.clone())
                };
                let body = value.to_string();
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        (
            Settings {
                api_base_url: address,
                ..Settings::default()
            },
            worker,
        )
    }

    #[test]
    fn permission_and_heartbeat_follow_real_http_response_contracts() {
        tauri::async_runtime::block_on(async {
            let credential = Credential {
                token: "fixture-test-only".into(),
                tenant: "0".into(),
            };
            for (status, response, allowed, expired) in [
                (
                    200,
                    json!({"success": true, "result": {"auth": [{"action": "system:internalLoginByAdmin", "type": "1"}]}}),
                    true,
                    false,
                ),
                (
                    200,
                    json!({"success": true, "result": {"auth": [], "allAuth": [{"action": "system:internalLoginByAdmin"}]}}),
                    false,
                    false,
                ),
                (
                    500,
                    json!({"success": false, "message": "Token失效，请重新登录"}),
                    false,
                    true,
                ),
            ] {
                let (settings, server) =
                    mock_api(status, response, "/sys/permission/getUserPermissionByToken");
                let result = check_permission(&settings, &credential).await;
                assert_eq!(result.is_ok(), allowed);
                if expired {
                    assert!(matches!(result, Err(AppError::Unauthorized)));
                } else if !allowed {
                    assert!(matches!(result, Err(AppError::Forbidden)));
                }
                server.join().unwrap();
            }
            let (settings, server) = mock_api(
                200,
                json!({"success": true, "result": {}}),
                "/sys/sysConfig/getByCode?code=SYSTEM_NOTIFICATIONS",
            );
            assert!(heartbeat(&settings, &credential).await.is_ok());
            server.join().unwrap();
        });
    }
    #[test]
    fn only_explicit_user_grant_allows_the_tool() {
        let grant = json!({"action": "system:internalLoginByAdmin", "type": "1"});
        assert!(has_internal_login_permission(
            &json!({"auth": [grant.clone()]})
        ));
        assert!(!has_internal_login_permission(
            &json!({"allAuth": [grant], "auth": []})
        ));
        assert!(!has_internal_login_permission(
            &json!({"auth": [{"action": "system:internalLoginByAdmin", "type": "2"}]})
        ));
        assert!(!has_internal_login_permission(
            &json!({"auth": [{"action": "system:internalLoginByAdminExtra"}]})
        ));
        assert!(!has_internal_login_permission(&Value::Null));
    }

    #[test]
    fn detects_http_and_jeecg_business_expiry_without_confusing_network_errors() {
        for (status, body) in [
            (401, json!({})),
            (200, json!({"code": "401"})),
            (500, json!({"message": "Token失效，请重新登录"})),
        ] {
            assert!(matches!(
                response_error(status, &body),
                Some(AppError::Unauthorized)
            ));
        }
        assert!(matches!(
            response_error(200, &json!({"code": 403})),
            Some(AppError::Forbidden)
        ));
        assert!(response_error(503, &json!({"message": "服务暂不可用"})).is_none());
    }
    #[test]
    fn matches_langu_supplied_signature_without_reusing_credentials() {
        let params = BTreeMap::from([("username".into(), "董永辉(A1186)".into())]);
        assert_eq!(signature(&params), "3913DEC17FA6913759FBFA84757ACF42");
    }
}
