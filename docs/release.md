# 安装包与发布

GitHub 仓库：`mrdong916/langu-account-tool`，主分支 `main`。
工作流：`.github/workflows/build.yml`（Build desktop installers）。

## 触发与下载

- 推送到 `main`：自动构建各平台安装包。
- GitHub 仓库 Actions → Build desktop installers → Run workflow：手动选择分支构建。
- 构建完成后，在运行详情页的 Artifacts 下载 `installers-<Rust target>`；压缩包内为安装包，保留 30 天。
- 推送 `v<版本>` 标签：所有构建成功后生成草稿 GitHub Release，并附上全部安装包及 `SHA256SUMS.txt`。不会自动公开发布。
- `check.yml` 在 push / pull_request 时运行三平台类型、lint、测试、格式及 Clippy 检查；拉取请求不触发安装包发布。

## 构建矩阵

| 平台 | 架构 | Runner | 安装包 |
| --- | --- | --- | --- |
| Windows | x64 | windows-2022 | NSIS .exe、.msi |
| Windows | x86 | windows-2022 | NSIS .exe、.msi |
| Windows | ARM64 | windows-2022 交叉编译 | NSIS .exe |
| macOS 14+ | Intel + Apple Silicon 通用版 | macos-14 | .dmg |
| Linux | x64 | ubuntu-22.04 | .deb、.rpm、.AppImage |
| Linux | ARM64 | ubuntu-24.04-arm | .deb、.rpm、.AppImage |

ARM64 Linux 安装包在 Ubuntu 24.04 构建，需匹配其 glibc / WebKitGTK 运行环境，不能承诺在所有旧发行版上运行。
GitHub 组织策略、仓库套餐和 Actions 额度可能限制 runner 使用，特别是 ARM64 runner；若任务长期排队，检查仓库设置或配置同架构自托管 runner。
Windows ARM32、macOS 32 位、Linux ARMv7/x86 及其他 CPU 架构不在本工作流中。

## 版本发布

1. 同步更新 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 的版本，并更新对应 lockfile。
2. 提交修改，确认 main 的 Project checks 与安装包构建通过。
3. 在已验证提交上创建并推送标签，例如当前版本使用 `v0.1.0`。
4. 验证各平台安装、启动、主/子账号隔离和开发者工具，再由维护者发布草稿 Release。

`scripts/check-release-version.mjs` 校验三个版本一致，标签必须严格等于 `v<版本>`。
同一标签重跑仅更新草稿资源；工作流拒绝覆盖已经公开发布的 Release。
未创建版本标签时，仅生成 Actions artifacts，不生成 Release。

## 签名与权限

当前没有配置 Windows 公司代码签名证书，也没有 Apple Developer ID 证书或公证凭据。
Windows 包未做公司签名；macOS 不具有公司 Developer ID 签名/公证，系统可能阻止打开。正式分发前需要接入公司签名流程，不建议关闭系统安全保护。
开发者工具使用 WebKit 私有 API，不面向 Mac App Store 发布。

构建 job 只有 `contents: read`；仅标签发布 job 获得 `contents: write`，使用 GitHub 自带的短期 `GITHUB_TOKEN`。
不需要在仓库保存主账号 token、SSH 私钥或长期 GitHub PAT。
代码保留 Codeup 远程；GitHub Actions 只在 GitHub 仓库执行，不自动同步 Codeup。
