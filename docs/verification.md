# 首版验证记录

日期：2026-10-13；环境：Windows x64 / WebView2。

## macOS 地址栏与子账号登录修复（2026-10-14）

- macOS 26.4 / Apple Silicon 原生测量：主 WebView 高度 900，而 CSS 视口高度 868，标题栏安全区域为 32。子 WebView 定位改为读取当前父视图的 `safeAreaInsets`，兼容窗口缩放与侧栏宽度变化，不写死标题栏高度。
- 本地测试系统新增 `/app` 延迟登录组件场景：未登录时延迟清 token，同时延迟下一次页面响应；旧实现的原生测试失败，复现子账号注入后又被清理。
- macOS 先加载没有业务脚本的同源空白文档，完成一次性缓存注入再打开业务页。新增和一键重登共用此路径，仍在独立账号数据存储内，并继续校验完整 origin 与顶层页面。
- 原生回归覆盖 1360×900 / 960×700 窗口与展开/折叠侧栏坐标、子账号登录及服务端校验、主子账号身份与草稿隔离、退出后刷新不重放凭据、一键重登后草稿保留。
- macOS 发布矩阵拆为 Intel `_x64.dmg` 与 Apple Silicon `_aarch64.dmg`；发布前检查两种 DMG 都存在。Intel 运行行为仍需对应设备验收。
- 在临时 GitHub 草稿实测中文附件上传与重命名均被清理；草稿已删除。改用中文显示标签和 UTF-8 ZIP，内部安装包保留“兰谷多账号工具”名称，下载 ZIP 使用 ASCII 版本/架构名。

## macOS 登录页闪退修复（2026-10-14）

- macOS 26.4 原生复现：HTTP 登录页被 ATS 拒绝（`-1022`）后，Wry 0.57.0 的 `url_from_webview` 对空 `WKWebView.URL` 执行 `unwrap()`，后台登录检查使整个应用退出。
- `Info.plist` 为 `languwin.com` 及其子域添加 HTTP 例外；其他域名仍遵循系统 ATS，HTTPS 证书校验不变。自定义 HTTP 域名需要在安装包中单独配置例外。
- 登录读取、清理和当前地址检查改用可超时的脚本求值，避免原生空 URL；登录缓存脚本仍校验完整 origin 和顶层页面。
- 打包后的原生冒烟测试通过：ATS 拦截首次导航时不退出；兰谷 HTTP 登录页完成加载；本地 test.admin / test.user.01 身份与草稿保持隔离。
- TypeScript、ESLint、9 项 Node 测试、前端生产构建、Rust 格式、7 项 Rust 测试及 Clippy 通过。

原生回归入口为 `src-tauri/examples/webview_smoke.rs`，只接受独立的 `com.langu.accounttool.smoke` 标识，不读取已保存账号。
在工程根目录启动本地测试系统（保持运行）：

```sh
LANGU_FIXTURE_PORT=1432 npm run test:system
```

另一终端构建测试程序并创建带正式 ATS 配置的测试 app：

```sh
npm run build:frontend
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
# 2026-10-14 工作区优化验收

- TypeScript、ESLint、11 项 Node 测试、前端生产构建、Rust 格式、10 项 Rust 测试、Clippy 通过。
- 原生 macOS 独立 smoke profile 使用 245 个本地用户完成多页全量同步，验证中文姓名、全拼、首字母、工号和部门组合检索。
- 模拟重新加载缓存后，以创建时间和更新时间分别拉取新增/修改；新用户没有 updateTime 也能出现。手动全量校准移除删除用户。
- 第二页故意返回 503，确认旧快照人数和同步游标不变，重试成功后再提交。
- 主子账号独立草稿、子账号退出不重放凭据、重新登录回原 profile、macOS 标题栏与原生子 WebView 边界回归通过。
- 透明提示页在原生应用中显示目标姓名，未抢占主窗口焦点；隐藏/显示窗口后子账号草稿保留。
- `pnpm dev:attached` 实际启动独立开发 profile；`pnpm build:mac:arm64 --debug --ci` 生成中文 DMG，并复制到 `work/installers/aarch64-apple-darwin/debug/`。
- Windows/Linux 的命令映射和错误宿主检查通过自动测试；本机不能验证其安装器与原生运行效果，需相应平台或 GitHub Actions 验证。线上用户目录契约未使用真实账号联调。

## 2026-10-14 运行入口对齐

- 按 nacos-control-panel 统一为 `tauri:dev` 和 `tauri:build:*`，更新当前开发文档；上节中的旧命令为当时的验收记录。
- `pnpm tauri:dev` 实际自动启动 Vite、完成 Rust 编译并启动 macOS 桌面进程，前端 HTTP 响应正常。
- `pnpm tauri:build:mac:arm --dry-run` 确认 ARM 目标和 APP/DMG 参数；此轮未重新生成安装包。
- `pnpm check`（10 项 Node 测试）、`pnpm build:frontend`、`cargo fmt --check`、`cargo test`（10 项 Rust 测试）通过。

## 2026-10-14 切换为 npm

- 本地脚本、Tauri 前置命令和 GitHub Actions 统一使用 npm；锁文件改为 `package-lock.json`，直接依赖版本保持不变。前文 pnpm 命令仅为历史验收记录。
- 从 npm 官方源生成锁文件，`npm ci` 安装成功；锁文件包含发布矩阵所需的各平台 Tauri CLI 包。
- `npm run check`（10 项 Node 测试）、`npm run build:frontend`、Rust 格式检查与 10 项测试、Release ZIP 命名测试通过。
- `npm run tauri:dev` 使用临时独立标识和 1433 端口验证，自动运行 npm 前端命令并启动 macOS 桌面进程；未停止用户原有的 1430 开发服务。
- ARM 打包命令 dry-run 与 Tauri CLI 入口检查通过；此轮未重新生成各平台安装包。

## 2026-10-14 正常账号目录与原生窗口验收

- 所有目录请求携带 `status=1`；增量另取 `field=id` 校准集合。新增 Rust 测试覆盖查询参数、旧缓存非正常账号过滤及正常快照落盘。
- 原生本地回归通过：245 个正常用户之外的冻结、离职、未知状态账号不出现；正常转冻结后增量移除，恢复正常后重新出现。分页失败保留快照与游标，主子账号会话、草稿和权限隔离仍通过。
- macOS 独立 QA 应用实测数据库入口、搜索页、`zs` 拼音检索、部门显示、翻页、行右侧单击打开及已打开账号状态；未出现确认弹窗，原生账号页面在目录页隐藏。
- 实测原生标题栏双击从 1100×760 放大再还原，并执行标题栏拖动；普通与放大窗口下地址栏完整可见，浏览器延伸到右侧及底部。
- 真实已登录应用仅查看页面和本地缓存状态统计；未通过真实账号提交业务、代登录、退出或修改系统设置。用户原有运行进程保留。
- `npm run check`（10 项 Node 测试）、前端构建、Rust 格式、12 项 Rust 测试与 Clippy 通过。生成 macOS ARM debug QA APP 完成界面验收；Windows/Linux 原生窗口仍需相应平台验证。

## 2026-10-14 加号搜索弹窗与打开性能

- 搜索页和数据库入口移除，点击子账号旁加号打开搜索弹窗；菜单、托盘和快捷键共用该弹窗，空关键词默认分页展示全部正常用户。
- 主账号和折叠按钮改为完整卡片背景；macOS 原生检查展开、折叠状态，头像、角色角标和加号均可见。
- 独立 QA 应用连接本地 1432 测试系统：245 个正常用户、`zs` 检索张三、清空输入恢复列表、翻页、手动刷新更新部门、直接打开和已打开账号切换通过。
- 连续打开和检索前后，本地服务端权限请求均为 3 次、用户列表请求均为 12 次，证明打开/搜索不再强制发起远程权限或目录请求。真正打开账号和手动刷新仍校验服务端权限。
- 修复原生视图显隐依赖 `requestAnimationFrame`：后台 macOS WebView 暂停动画帧时，关闭弹窗或切换账号曾无法即时恢复页面。现在直接提交显隐和尺寸更新，保留重复尺寸去重与顺序队列；新增无动画帧回归测试。
- 修复后原生实测 Esc 关闭立即恢复主账号及其草稿，单击直接打开立即呈现独立子账号页面。测试系统拒绝权限后，弹窗关闭、加号与子账号禁用，未发送目标账号代登录请求；恢复权限后主账号草稿仍在。
- `npm run check`（12 项 Node 测试）、`npm run build:frontend`、`cargo fmt --check` 与 12 项 Rust 测试通过。全部账号操作仅使用独立本地测试账号，未对真实已登录账号执行写入或代登录操作。
