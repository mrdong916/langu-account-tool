# 开发规范

需求和验收记录在 docs/product.md；架构决策记录在 docs/architecture.md。
接口变更先更新 docs/api.md，并补充能覆盖协议行为的测试。

## 环境

使用 Node.js 22.13+、项目指定 pnpm、Rust stable。
Windows 需要 MSVC C++ build tools 与 WebView2 Runtime。
Linux/macOS 依赖和架构说明见 docs/platforms.md。

```sh
pnpm install --frozen-lockfile
pnpm tauri:dev
```

桌面开发：`pnpm tauri:dev`，自动启动前端和桌面窗口。
前端已通过 `pnpm start` 单独运行时：`pnpm tauri:dev:attached`。
也可以停止前端预览后直接运行 `pnpm tauri:dev`，避免固定开发端口 1430 被占用。

```sh
pnpm check
pnpm build:frontend
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## 提交与分支

- main 为稳定分支，功能分支 feat/<module>，修复分支 fix/<issue>。
- 每次提交聚焦一个可审查变更，使用 Conventional Commits。
- PR 记录行为变化、验证结果及平台限制。界面改动附截图。
- 没有业务需求时不预建空服务、数据库或大型 monorepo 子包。
- 前端格式交给 Prettier；ESLint 检查语义；Rust 使用 rustfmt 和 Clippy。

## 自动发布

GitHub Pull Request 会执行三平台检查。推送到 GitHub `main` 后，工作流通过检查再构建 Windows x64/x86/ARM64、Linux x64/ARM64 和 macOS 通用安装包，所有平台成功后自动创建并发布 GitHub Release。
Release 标签包含应用版本、运行编号、重跑次数和提交号；不需要先手工打 tag，也不会自动提交版本号变更。手动重跑使用新的 Release 标签。
工作流和发布权限说明见 [README](README.md#github-自动构建与发布)。推送到 Codeup 本身不会触发这套 GitHub 工作流。

## 本地会话验收

`pnpm test:system` 启动 127.0.0.1:1431 的纯本地测试系统。
在桌面客户端将登录地址与接口地址均设为 `http://127.0.0.1:1431`。
测试页面自动建立虚拟主账号，检索提供 45 个虚拟用户，用于验证分页和切换。
测试服务不连接线上，不包含真实凭据，也不会随应用打包。

验收主账号、两个子账号分别输入不同草稿，切换、刷新、关闭和重新打开。
检查三者页面草稿独立、主账号身份始终不变，以及子账号退出后不会自动重放 token。
检查弹窗期间原生 WebView 不遮挡弹窗，取消后页面可继续使用。
