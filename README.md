# 兰谷多账号工具

面向兰谷产品、运营和测试人员的桌面账号工作区。
使用 Rust + Tauri 2 + Vue 3，支持主账号登录、检索用户、授权代登录及独立会话切换。

GitHub：`git@github.com:mrdong916/langu-account-tool.git`，主分支 `main`。
Codeup：`git@codeup.aliyun.com:6889be5d349fe2cce7f70454/tool/langu-account-tool.git`。
公司 Logo 源自 http://languwin.com/logo.png，随程序本地打包；图标更新执行 `scripts/generate-icon.ps1`。
窗口、应用和安装器使用公司图标；侧栏主账号入口使用账号头像。

## 快速开始

需要 Node.js 22.13+、Node 自带的 npm、Rust stable，以及对应平台的 Tauri 系统依赖。

```sh
npm install
npm run tauri:dev
```

开发和打包入口沿用 nacos-control-panel 的 `tauri:dev` / `tauri:build:*` 命名。
`npm run tauri:dev` 自动启动前端和桌面窗口，无需额外开一个终端。
浏览器界面预览：`npm start`，地址 http://127.0.0.1:1430/。
原生账号页面仅在桌面程序运行，浏览器预览不会伪造登录或通过 iframe 模拟隔离。

## 使用

1. 设置系统登录地址、接口地址、子账号登录后地址，保存并打开主账号。
2. 在系统自己的登录页面完成密码、扫码或验证码登录；客户端识别当前账号。
3. 主账号通过 system:internalLoginByAdmin 权限校验后，点击左侧加号检索系统用户；无权限则锁定工作区。
4. 搜索弹窗默认分页展示全部已缓存的正常用户；点击行右侧“直接打开”，调用服务端授权代登录接口，在独立会话打开目标用户；已打开的账号直接切换。
5. 左侧主账号与子账号分组显示，点击切换，页面状态保留。
6. 关闭工作区可以保留主账号登录状态，或勾选清除本地登录状态。
7. 主账号过期后回到登录页并锁定子账号；子账号过期可点击其左侧重登入口恢复。
8. 点击子账号旁的加号弹出搜索，支持浏览、检索和手动刷新用户缓存；菜单、托盘及 `⌘/Ctrl+Shift+K` 打开同一弹窗。账号侧栏可折叠，切换账号显示透明大字提示。
9. 主账号登录后只缓存正常状态的可见用户，重启增量同步并校准正常账号集合，移除冻结、离职或已删除的用户。支持姓名、工号、部门、拼音全拼和首字母即时检索。
10. 关闭窗口只隐藏到托盘；从应用菜单或托盘选择“退出”才结束程序。

## 目录

```text
langu-account-tool/
  .github/workflows/   多平台检查、安装包构建和 GitHub Release 发布
  docs/               产品、架构、协议、平台矩阵
  scripts/            开发/打包入口、本地测试系统
  tests/              登录缓存协议测试
  src/
    app/              应用入口
    api/              类型化的客户端调用入口
    assets/           内置公司 Logo 等资源
    components/       账号头像、用户检索弹窗等组件
    hooks/            原生浏览区域生命周期
    layouts/          工作区布局入口
    router/           Hash 路由
    services/         Tauri 平台封装
    stores/           Pinia 状态和操作编排
    styles/           全局样式
    types/            领域类型
    views/            设置和账号工作区
  src-tauri/
    capabilities/     本地界面权限
    scripts/          受 origin 限制的登录缓存适配
    src/
      commands/       IPC 边界、来源校验
      domain/         会话、账号、配置及校验规则
      services/       WebView 会话生命周期
      infra/          配置持久化、系统 HTTP 接口
    Cargo.lock
  AGENTS.md
  CONTRIBUTING.md
  package-lock.json
```

遵循国内 Vue 管理端项目常见模块化目录，并为 Rust 原生能力单独分层。
用户目录通过 Rust + SQLite 缓存，登录凭据不会进入用户目录数据库。

## 验证与打包

```sh
npm run check
npm run build:frontend
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

平台范围见 [docs/platforms.md](docs/platforms.md)，接入协议见 [docs/api.md](docs/api.md)。
各平台调试、Intel/ARM 打包命令及系统依赖见 [docs/development.md](docs/development.md)。

## GitHub 自动构建与发布

`.github/workflows/build.yml` 在每次推送到 GitHub 的 `main` 分支后执行检查、构建安装包，并自动发布到该仓库的 Releases：

- Windows x64 / x86：`.exe`（NSIS）和 `.msi`；ARM64：`.exe`。
- Linux x64 / ARM64：`.deb`、`.rpm` 和 `.AppImage`。
- macOS 14+：Intel 和 Apple Silicon 分别提供 `.dmg`。

检查由可复用的 `check.yml` 执行；构建矩阵和下载说明见 [docs/release.md](docs/release.md)。
全部平台检查和构建成功后才统一发布；上传阶段使用草稿，安装包上传成功后自动公开，无需手工确认。
每次执行使用独立标签 `v<应用版本>-build.<运行编号>.<重跑次数>-<短提交号>`，例如 `v0.1.0-build.12.1-5c578d9`，不会覆盖历史版本。
标签指向本次构建的提交；安装包内部版本沿用项目配置，发布新的应用版本时需同步更新 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 和锁文件。

Pull Request 只执行检查，不发布 Release；其他分支推送不触发发布。也可在 Actions 的 **Build and release** 中选择 `main` 手动运行。
发布使用 GitHub 自动提供的 `GITHUB_TOKEN`，仅发布任务申请 `contents: write`，不需要额外配置个人访问令牌。仓库或组织须允许运行这些 Actions 及写入 Releases。

当前本地 `origin` 指向 Codeup。上述自动化需要代码实际推送或镜像同步到 GitHub 仓库；只推送 Codeup 不会触发 GitHub Actions。
`scripts/release.ps1` 仍用于本地检查和打包。自动构建未配置发行证书签名或 macOS 公证，平台运行与隔离能力仍需按 [平台矩阵](docs/platforms.md) 完成真机验收。
