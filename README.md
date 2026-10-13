# 兰谷多账号工具

面向兰谷产品、运营和测试人员的桌面账号工作区。
使用 Rust + Tauri 2 + Vue 3，支持主账号登录、检索用户、授权代登录及独立会话切换。

代码仓库：`git@codeup.aliyun.com:6889be5d349fe2cce7f70454/tool/langu-account-tool.git`，主分支 `main`。
公司 Logo 源自 http://languwin.com/logo.png，随程序本地打包；图标更新执行 `scripts/generate-icon.ps1`。

## 快速开始

```sh
pnpm install --frozen-lockfile
pnpm dev
```

浏览器界面预览：`pnpm dev:frontend`，地址 http://127.0.0.1:1430/。
原生账号页面仅在桌面程序运行，浏览器预览不会伪造登录或通过 iframe 模拟隔离。

## 使用

1. 设置系统登录地址、接口地址、子账号登录后地址，保存并打开主账号。
2. 在系统自己的登录页面完成密码、扫码或验证码登录；客户端识别当前账号。
3. 主账号通过 system:internalLoginByAdmin 权限校验后，点击左侧加号检索系统用户；无权限则锁定工作区。
4. 选择用户并确认，调用服务端授权代登录接口，在新会话打开目标用户。
5. 左侧主账号与子账号分组显示，点击切换，页面状态保留。
6. 关闭工作区可以保留主账号登录状态，或勾选清除本地登录状态。
7. 主账号过期后回到登录页并锁定子账号；子账号过期可点击其左侧重登入口恢复。
8. 侧栏可折叠为 68px，底部提供设置和关闭工作区，开发工具位于首页图标之后。

## 目录

```text
langu-account-tool/
  .github/workflows/   多平台检查流水线
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
  pnpm-lock.yaml
```

遵循国内 Vue 管理端项目常见模块化目录，并为 Rust 原生能力单独分层。
尚未加入无业务依据的数据库、自动升级或额外系统插件。

## 验证与打包

```sh
pnpm check
pnpm build:frontend
cargo test --manifest-path src-tauri/Cargo.toml
pnpm build
```

平台范围见 [docs/platforms.md](docs/platforms.md)，接入协议见 [docs/api.md](docs/api.md)。
`.github/workflows/check.yml` 是 GitHub Actions 格式；当前 Codeup 仓库尚未配置云效流水线，可使用 `scripts/release.ps1` 运行本地发布检查。
正式签名、安装包与多架构真机验证需按发布矩阵完成；当前并未宣称所有平台已经验证。
