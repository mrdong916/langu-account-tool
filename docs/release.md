# 安装包与发布

GitHub 仓库：`mrdong916/langu-account-tool`，主分支 `main`。
工作流：`.github/workflows/build.yml`（Build and release），复用 `check.yml` 执行发布前检查。

## 触发与下载

- 推送到 `main`：先执行三平台检查，再构建各平台安装包；全部成功后自动发布 GitHub Release。
- GitHub 仓库 Actions → Build and release → Run workflow：手动选择分支构建；选择 `main` 时也会自动发布，其他分支只构建。
- 构建完成后，在运行详情页的 Artifacts 下载 `installers-<Rust target>`；压缩包内为安装包，保留 30 天。
- GitHub Releases 提供全部安装包及 `SHA256SUMS.txt`。上传时先创建草稿，上传成功后自动公开，无需维护者手工发布。
- Release 标题只显示版本标签。附件统一使用 `langu-account-tool` 前缀（例如 `langu-account-tool_0.1.0_universal.dmg`），避免 GitHub 清理中文文件名时丢失软件名称；校验清单使用最终附件名。
- `check.yml` 在 pull_request、手动执行或由构建工作流调用时运行三平台类型、lint、测试、格式及 Clippy 检查；拉取请求不触发安装包发布。
- 其他分支和标签推送不触发发布；连续推送不会取消较早提交的构建。

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

Windows MSI 使用 `bundle.windows.wix.language = "zh-CN"`，匹配中文产品名所需的代码页；NSIS 的语言配置独立生效。
构建开启 `--verbose`，WiX 链接失败时可在 Actions 日志查看 `light.exe` 的具体诊断。

## 版本发布

1. 提交修改并推送到 GitHub `main`，无需先手工创建版本标签。
2. 工作流校验应用版本一致性并执行检查、安装包构建。
3. 全部构建成功后，自动创建标签 `v<应用版本>-build.<运行编号>.<重跑次数>-<短提交号>`，例如 `v0.1.0-build.12.1-5c578d9`，指向本次构建的提交。
4. 安装包上传完成后自动发布 Release；各平台安装、启动、主/子账号隔离和开发者工具仍需真机验收。

`scripts/check-release-version.mjs` 校验 `package.json`、`src-tauri/tauri.conf.json` 和 `src-tauri/Cargo.toml` 三个版本一致。
自动发布不修改安装包内部版本，也不自动提交版本号变更；更新应用版本时需同步修改这三个文件并更新对应 lockfile。
每次执行及重跑使用独立 Release 标签，不覆盖历史版本。检查、构建或上传失败时不公开发布；上传失败可能留下草稿，修复后重新运行即可。

## 签名与权限

当前没有配置 Windows 公司代码签名证书，也没有 Apple Developer ID 证书或公证凭据。
Windows 包未做公司签名；macOS 不具有公司 Developer ID 签名/公证，系统可能阻止打开。正式分发前需要接入公司签名流程，不建议关闭系统安全保护。
开发者工具使用 WebKit 私有 API，不面向 Mac App Store 发布。

检查与构建 job 只有 `contents: read`；仅发布 job 获得 `contents: write`，使用 GitHub 自带的短期 `GITHUB_TOKEN`。
不需要在仓库保存主账号 token、SSH 私钥或长期 GitHub PAT。
代码保留 Codeup 远程；GitHub Actions 只在 GitHub 仓库执行，不自动同步 Codeup。
