# 本地调试与打包

需要 Node.js 22.13+、package.json 指定版本的 pnpm、Rust stable。
执行 `pnpm install --frozen-lockfile` 后使用以下命令。
使用 nvm 时先执行 `nvm use`；本项目的 `.nvmrc` 指定 Node 22。

| 用途 | 命令 |
| --- | --- |
| 当前系统桌面热更新调试 | `pnpm dev` |
| Mac / Windows / Linux 调试 | `pnpm dev:mac` / `pnpm dev:win` / `pnpm dev:linux` |
| 仅预览前端 | `pnpm dev:frontend` |
| 连接已经启动的前端服务 | `pnpm dev:attached` |
| 当前系统、当前架构安装包 | `pnpm build` |
| Mac 当前架构 / ARM64 / Intel / Universal | `pnpm build:mac` / `pnpm build:mac:arm64` / `pnpm build:mac:intel` / `pnpm build:mac:universal` |
| Windows x64 / x86 / ARM64 | `pnpm build:win` / `pnpm build:win:x86` / `pnpm build:win:arm64` |
| Linux 当前架构 / x64 / ARM64 | `pnpm build:linux` / `pnpm build:linux:x64` / `pnpm build:linux:arm64` |

开发模式标识为 `com.langu.accounttool.dev`，设置、用户缓存和 WebView profile 与正式版本分离。前端修改热更新，Rust 修改重新编译。
账号页面工具栏的扳手打开该账号 WebView 的开发者工具。

构建结果会复制到 `work/installers/<Rust目标>/release/`，文件名使用“兰谷多账号工具”。
原始 Tauri 产物位于 `src-tauri/target/<Rust目标>/release/bundle/`。
可加 `--debug` 生成包含调试信息的安装包，对应目录为 `debug`；例如 `pnpm build:mac:arm64 --debug --ci`。
可加 `--dry-run` 只检查当前平台并显示命令。

## 系统依赖与架构

- macOS：安装 Xcode Command Line Tools（`xcode-select --install`）。运行最低 macOS 14。使用 rustup 时，Intel 构建需要 `rustup target add x86_64-apple-darwin`，ARM64 需要 `rustup target add aarch64-apple-darwin`；Universal 两个目标都需要。Homebrew Rust 可直接构建其已安装标准库的架构。
- Windows：安装 Visual Studio Build Tools 的 C++ 桌面开发工作负载、Windows SDK、WebView2。x86/ARM64 还需要对应 Rust 标准库和 MSVC 编译工具。x64/x86 输出 NSIS EXE 与 MSI；ARM64 输出 NSIS EXE。
- Linux（Ubuntu/Debian）：安装 `build-essential curl wget file libwebkit2gtk-4.1-dev libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf desktop-file-utils`。输出 DEB、RPM、AppImage。建议在目标架构 Linux 本机或虚拟机中构建；仅添加 Rust target 不会提供交叉编译所需的 GTK/WebKit 系统库。

macOS、Windows、Linux 安装包需要在对应操作系统构建；脚本会拒绝错误系统，不会假装在 Mac 上生成 Windows/Linux 安装包。GitHub Actions 继续负责完整七目标矩阵。
本地打包不需要 GitHub 凭据；发行签名和 macOS 公证按已有 Tauri 环境变量配置。

## 本地虚拟账号回归

`pnpm test:system` 启动本地系统（默认 1431 端口）。测试数据不属于线上系统，支持 245 个虚拟用户、部门与工号、分页与增量条件。
原生自动回归使用 1432 端口，步骤见 [verification.md](verification.md)。

```sh
pnpm check
pnpm build:frontend
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
```
