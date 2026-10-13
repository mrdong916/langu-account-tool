# 兰谷系统接口适配

协议依据用户提供的请求样例及同级 langu-web-oa 源码。
不保存样例中的临时 JWT，不在源代码或日志中复制真实凭据。

## 线上地址

- 登录页：http://languwin.com/
- API：http://api.languwin.com
- 用户搜索：GET /sys/user/list
- 代登录：POST /sys/internalLoginByAdmin，JSON body 为 `{ "username": "目标用户名" }`
- 可选服务器时间：GET /sys/common/getServerTimestamp
- 主账号权限：GET /sys/permission/getUserPermissionByToken；只检查 result.auth[].action 是否精确匹配 system:internalLoginByAdmin，并排除 type=2 的禁用项，绝不从 allAuth 授权。
- 登录保活：GET /sys/sysConfig/getByCode?code=SYSTEM_NOTIFICATIONS；主账号五分钟一次，子账号选择时及前台运行期间定期检查。

目前地址为用户指定的 HTTP，通信不会凭空变成加密连接。服务支持 HTTPS 后应在设置中同步替换。

## 请求签名

对应 langu-web-oa/src/utils/encryption/signMd5Utils.js：

1. 参数顶层 key 按字典序排序，数值转换为字符串。
2. JSON 序列化排序结果并拼接同协议签名后缀，计算 MD5，转换大写。
3. X-Sign 使用新签名；X-TIMESTAMP 使用 Asia/Shanghai 的 YYYYMMDDHHmmss。
4. 搜索/代登录/权限请求的 X-Access-Token 始终读取主账号 profile；tenant-id 读取其租户，缺省为 0。子账号有效性探测仅使用其自身 token 调用保活接口，不可调用管理接口。
5. GET 的 `_t` 由原站 Axios 拦截器在签名后添加，故不参与签名。

服务器时间接口失败时使用本机时间。HTTP 请求超时、401、403 和业务失败单独反馈。
搜索参数为 username=`*关键词*`、column=createTime、order=desc、pageNo、pageSize=20。
空关键词加载第一页，分页遍历用户；服务端权限仍然决定可检索范围。

## 响应契约

```json
{
  "success": true,
  "result": { "records": [{ "id": "...", "username": "...", "avatar": "..." }], "total": 100 }
}
```

代登录响应：

```json
{
  "success": true,
  "result": {
    "token": "<runtime-only>",
    "userInfo": { "id": "...", "username": "...", "avatar": "..." },
    "accounts": [],
    "sysAllDictItems": {},
    "erpUserInfo": []
  }
}
```

必须验证返回 username 与目标一致。以上不是线上返回结果的伪造记录，而是按现有前端消费逻辑确定的契约。

## 登录缓存

原系统使用 vue-ls 3.2，namespace=`pro__`，存储 JSON `{ "value": ..., "expire": 时间戳或null }`。

| key                        | 来源                                         |
| -------------------------- | -------------------------------------------- |
| pro__Access-Token          | result.token                                 |
| pro__Login_Username        | result.userInfo.username                     |
| pro__Login_Userinfo        | result.userInfo                              |
| pro__Login_settlementOrg   | result.accounts                              |
| pro__UI_CACHE_DB_DICT_DATA | result.sysAllDictItems                       |
| pro__Login_ErpUserList     | result.erpUserInfo                           |
| pro__TENANT_ID             | 主账号当前 tenant-id                         |
| pro__canLogin              | true                                         |
| USER_ID                    | result.userInfo.id，普通 localStorage 字符串 |

子账号缓存期限遵循原站七天行为；JWT/服务端过期仍然优先，客户端不延长凭据。
完整缓存写入后重新进入配置的落地地址，让原站自行加载路由、菜单和权限。

## 本地 IPC

workspace_init、save_settings、open_primary、sync_sessions、search_users、add_account、
select_session、browser_bounds、browser_action、close_session、close_workspace。
权限/恢复命令：check_primary_access、relogin_primary、relogin_session。
每个命令校验 main 来源；跨窗口显示信息通过仅发给 main 的 workspace-changed 事件同步。
settings 不接受 token 字段，搜索/代登录也不接受由 Vue 提交的管理 token。

## 失效与错误

HTTP 401、业务 code=401、Jeecg 的 HTTP 500 + message=Token失效均归类为登录过期；HTTP/业务 403 为拒绝授权。
网络超时与 5xx 普通故障不等于过期，不清除缓存。权限检查失败时默认锁定工作区，允许重试。
主账号过期会清除其登录缓存并回到配置的登录入口，但不清除业务草稿和子账号的独立存储。
保活不能绕过服务器的 token 有效期或强制注销。
