# 平台与架构

产品目标是 Windows、Linux、macOS 桌面。Rust 能编译某 CPU 架构不代表系统 WebView 和安装器均可用。
以下是计划支持范围，发布须通过对应真机测试；当前开发机器仅能直接验证 Windows x64。

| 平台          | 架构 / Rust target                    | 状态与约束                                               |
| ------------- | ------------------------------------- | -------------------------------------------------------- |
| Windows 10/11 | x64 / x86_64-pc-windows-msvc          | 首版开发验证目标，依赖 WebView2                          |
| Windows 10/11 | x86 / i686-pc-windows-msvc            | 候选构建目标，32 位地址空间限制会话数量，需真机验证      |
| Windows 11    | ARM64 / aarch64-pc-windows-msvc       | 候选构建目标，需 ARM64 WebView2 和设备验证               |
| Windows       | ARM32                                 | 不支持，不能承诺 WebView2 桌面运行时可用                 |
| macOS 14+     | x64 / x86_64-apple-darwin             | 候选目标，需独立构建/签名/真机验证                       |
| macOS 14+     | ARM64 / aarch64-apple-darwin          | 候选目标，需独立构建/签名/真机验证                       |
| macOS         | x86、ARM32                            | 不支持，现代 macOS 无 32 位桌面应用支持                  |
| Linux         | x64 / x86_64-unknown-linux-gnu        | 候选目标，需 GTK3、WebKitGTK 4.1                         |
| Linux         | ARM64 / aarch64-unknown-linux-gnu     | 候选目标，需对应发行版依赖与设备验证                     |
| Linux         | ARMv7 / armv7-unknown-linux-gnueabihf | 实验目标，依赖发行版 WebKitGTK/工具链可用性              |
| Linux         | x86 / i686-unknown-linux-gnu          | 实验目标，依赖发行版 WebKitGTK/工具链可用性              |
| 其他          | RISC-V、LoongArch 等                  | 尚未支持，逐项评估 Tauri/Wry、WebKitGTK、Rust 和打包链路 |

## 会话隔离

Windows/Linux 为每个账号分配独立 WebView 数据目录。
macOS 使用 WKWebsiteDataStore 标识，这个能力需要 macOS 14+，因此应用设置最低版本 14。
主账号可持久化；子账号使用独立临时会话，不在下次启动自动恢复。
子账号页面、Cookie、localStorage、IndexedDB 隔离效果需在每种平台真机验收。

## Linux 依赖

Ubuntu 22.04/24.04 示例：

```sh
sudo apt-get update
sudo apt-get install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf
```

## 打包

```sh
pnpm build
pnpm exec tauri build --target i686-pc-windows-msvc
pnpm exec tauri build --target aarch64-pc-windows-msvc
pnpm exec tauri build --target universal-apple-darwin
```

跨 target 前先通过 rustup 安装对应 target 并准备系统工具链。
不能在一台 Windows 机器上直接生成并验证所有 macOS/Linux 安装包。
GitHub Actions 提供三平台检查及多架构打包，具体矩阵与产物下载见 [release.md](release.md)。
本地通过不等于所有平台已发布；安装包生成也不等于会话隔离已经完成真机验收。

macOS 启用开发者工具涉及 WebKit 私有 API；本项目面向企业内部分发，不以 Mac App Store 为目标。
正式发布还需公司提供签名证书、公证信息并确认最终应用标识；公司图标已配置。
