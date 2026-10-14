# 首版验证记录

日期：2026-10-13；环境：Windows x64 / WebView2。

## macOS 登录页闪退修复（2026-10-14）

- macOS 26.4 原生复现：HTTP 登录页被 ATS 拒绝（`-1022`）后，Wry 0.57.0 的 `url_from_webview` 对空 `WKWebView.URL` 执行 `unwrap()`，后台登录检查使整个应用退出。
- `Info.plist` 为 `languwin.com` 及其子域添加 HTTP 例外；其他域名仍遵循系统 ATS，HTTPS 证书校验不变。自定义 HTTP 域名需要在安装包中单独配置例外。
- 登录读取、清理和当前地址检查改用可超时的脚本求值，避免原生空 URL；登录缓存脚本仍校验完整 origin 和顶层页面。
- 打包后的原生冒烟测试通过：ATS 拦截首次导航时不退出；兰谷 HTTP 登录页完成加载；本地 test.admin / test.user.01 身份与草稿保持隔离。
- TypeScript、ESLint、9 项 Node 测试、前端生产构建、Rust 格式、7 项 Rust 测试及 Clippy 通过。

原生回归入口为 `src-tauri/examples/webview_smoke.rs`，只接受独立的 `com.langu.accounttool.smoke` 标识，不读取已保存账号。
在工程根目录启动本地测试系统（保持运行）：

```sh
LANGU_FIXTURE_PORT=1432 pnpm test:system
```

另一终端构建测试程序并创建带正式 ATS 配置的测试 app：

```sh
pnpm build:frontend
TAURI_CONFIG='{"identifier":"com.langu.accounttool.smoke"}' cargo build --manifest-path src-tauri/Cargo.toml --example webview_smoke --features custom-protocol --locked
python3 - <<'PY'
import plistlib
import shutil
from pathlib import Path

contents = Path('work/WebView Smoke.app/Contents')
(contents / 'MacOS').mkdir(parents=True, exist_ok=True)
info = plistlib.loads(Path('src-tauri/Info.plist').read_bytes())
info.update(CFBundleExecutable='webview_smoke', CFBundleIdentifier='com.langu.accounttool.smoke', CFBundleName='Langu WebView Smoke', CFBundlePackageType='APPL')
(contents / 'Info.plist').write_bytes(plistlib.dumps(info))
shutil.copy2('src-tauri/target/debug/examples/webview_smoke', contents / 'MacOS/webview_smoke')
PY
LANGU_SMOKE_MODE=blocked 'work/WebView Smoke.app/Contents/MacOS/webview_smoke'
LANGU_SMOKE_MODE=local 'work/WebView Smoke.app/Contents/MacOS/webview_smoke'
LANGU_SMOKE_MODE=remote 'work/WebView Smoke.app/Contents/MacOS/webview_smoke'
```

每次应输出 `WebView smoke passed` 并返回 0。`remote` 仅访问公开登录页，不登录或调用线上管理接口。

## 折叠入口、图标与安装包工作流

- 展开状态下折叠箭头并入主账号信息右侧，折叠状态下置于头像下方；保留键盘焦点、提示及展开状态标记。
- 公司图标用于 Windows 可执行文件、原生窗口和 NSIS 安装/卸载器，构建脚本显式监听 icons 目录变化，避免继续复用旧图标资源。
- 新增 GitHub Actions 多架构安装包构建与标签草稿 Release，独立 actionlint 检查通过。
- 新版 Node.js 下将测试入口限定为 tests/*.test.mjs，避免目录入口错误或误运行 scripts/test-system.mjs。
- pnpm 冻结安装、前端检查与 9 项 Node 测试、7 项 Rust 测试、Rust 格式及 Clippy 检查通过。
- 本地 Windows x64 debug NSIS 安装包成功生成；不代表其他平台云端构建已成功或安装器已完成安装验收。

## 主题与主账号展示更新

- 客户端主题改为 #1890ff，按钮、复选框、焦点与账号选中态使用统一主题变量；错误和过期状态保留语义色。
- 侧栏顶部 Logo 与重复主账号条目合并为主账号入口，显示真实头像和“主”角标。展开显示姓名与状态，折叠只显示头像。
- 客户端侧栏、悬浮提示、底部状态栏和已记住账号提示不再展示主账号 username；业务网页自身展示不修改。
- 新增 5 项界面渲染回归测试，覆盖头像与角标、用户名隐藏、姓名回退、切换子账号、未登录和过期状态；Node 合计 9 项通过。
- TypeScript、ESLint、前端生产构建、Rust 格式检查与 7 项 Rust 测试通过。
- 1360px 与 390px 预览检查通过：折叠宽度 68px，主账号文字隐藏，角标未裁切，页面无横向溢出。
- Windows x64 内置资源 debug 版本已重新构建并重启，原生窗口确认头像显示、侧栏折叠和主账号登录恢复正常。

## 权限与布局更新验收

- 新增权限/保活 HTTP 契约测试、登录过期分类、token 授权缓存绑定和保活间隔测试；Rust 合计 7 项通过。
- 新增登录缓存清理范围测试；Node 合计 4 项通过。
- 独立本地测试 profile 与本地 1432 测试系统完成原生验收，不使用线上账号进行失效模拟。
- 用户授权缺失但 allAuth 含代登录权限时显示“暂无权限”，原生页面隐藏，子账号切换和重登被锁定。
- 主账号服务端返回 HTTP 500 + Token失效后，点击添加不打开搜索，自动回到主账号登录页；子账号列表仍保留且禁用。
- 重新登录并恢复权限后工作区解锁，原有子账号列表保留。
- 子账号服务端失效后重新选择可显示“登录已过期”；一键重登后恢复 test.user.01，未创建重复账号。
- 本地请求记录确认权限、搜索和代登录使用 test.admin，子账号有效性探测仅调用配置接口。
- 本地后台两次连续主账号保活间隔 300.410 秒，验证五分钟保活调度实际生效。
- 公司 Logo 在桌面与 390px 预览中正确加载；折叠栏宽度实测 68px，无页面横向溢出。
- 最新界面移除整个顶部横栏，侧栏折叠为 68px，Logo 下方显示展开按钮，底部保留设置与关闭。
- Windows x64 内置资源 debug 版本已构建并重启；不代表正式签名发行版或其他平台真机通过。

## 自动检查

- TypeScript 类型检查、ESLint 通过。
- 前端生产构建通过。
- Node 登录缓存协议测试 3 项通过：缓存完整性与隔离、origin/iframe 限制、过期和损坏缓存。
- Rust 单元测试 2 项通过：URL 校验、签名与真实请求样例匹配。
- cargo clippy -- -D warnings 通过。
- Tauri 内置资源的 Windows x64 debug 构建通过；没有生成正式签名安装包。

## 原生程序验证

使用 `scripts/test-system.mjs` 的纯本地虚拟账号：

- 设置保存后打开主账号，识别 test.admin。
- 搜索 45 个用户，翻到第二页，按 test.user.23 检索得到唯一结果。
- 同时打开 test.user.01 和 test.user.23，右上角一直显示主账号。
- 在子账号页面再次检索和添加用户成功，说明管理请求仍使用主账号凭据。
- 主/子账号分别输入 MAIN-DRAFT 与 CHILD-DRAFT，切换后各自保留。
- 刷新主账号页面后 MAIN-DRAFT 仍保留。
- 子账号开发者工具成功打开独立的 InPrivate 调试窗口。
- 检索弹窗显示时原生 WebView 隐藏，弹窗内容没有被遮挡。
- 重新启动内置前端的独立程序后，主账号用户名、登录和 MAIN-DRAFT 恢复。
- 浏览器预览在 390px 和 1360px 宽度检查未发现页面横向溢出。

## 尚未验证

- 未使用用户消息中的 JWT 调用线上搜索和代登录；线上实际授权需用户登录后联调。
- 未验证 macOS/Linux、Windows x86/ARM64 的运行与隔离，参见 platforms.md。
- 未验证原站所有上传下载、弹出窗口、扫码登录、复杂重定向和内存压力场景。
- 账号数无业务限制，但没有做高并发会话容量测试。
