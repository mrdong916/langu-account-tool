# 本地调试与打包

需要 Node.js 22.13+、Node 自带的 npm、Rust stable。
执行 `npm install` 后使用以下命令。
使用 nvm 时先执行 `nvm use`；本项目的 `.nvmrc` 指定 Node 22。

命名沿用 nacos-control-panel；**本地运行只需要 `npm run tauri:dev`**，自动启动前端和桌面窗口。
也可以在 IDE 的 package.json 脚本列表里点击 `tauri:dev` 运行。

| 用途 | 命令 |
| --- | --- |
| 桌面调试（所有系统） | `npm run tauri:dev` |
| 仅预览前端 | `npm start` |
| 当前系统、当前架构安装包 | `npm run tauri:build` |
| Mac 当前架构 | `npm run tauri:build:mac` |
| Mac ARM / Intel / Universal | `npm run tauri:build:mac:arm` / `npm run tauri:build:mac:intel` / `npm run tauri:build:mac:universal` |
| Windows x64 / x86 / ARM64 | `npm run tauri:build:win` / `npm run tauri:build:win:x86` / `npm run tauri:build:win:arm` |
| Linux 当前架构 / x64 / ARM64 | `npm run tauri:build:linux` / `npm run tauri:build:linux:x64` / `npm run tauri:build:linux:arm` |

仅当前端已单独启动时，使用 `npm run tauri:dev:attached`。
`npm run dev` 和 `npm run build` 保留为兼容入口，分别调用 `tauri:dev` 和 `tauri:build`。

开发模式标识为 `com.langu.accounttool.dev`，设置、用户缓存和 WebView profile 与正式版本分离。前端修改热更新，Rust 修改重新编译。
账号页面工具栏的扳手打开该账号 WebView 的开发者工具。

构建结果会复制到 `work/installers/<Rust目标>/release/`，文件名使用“兰谷多账号工具”。
本地脚本在构建前仅清理本次格式对应的原始安装包目录，保留编译缓存和此前导出的安装包；构建成功后逐格式校验，缺失或出现多个安装包时停止复制，避免旧包覆盖新版。脚本不接受覆盖 target、bundles、profile 或 target-dir 的参数；需要自定义这些选项时直接使用 `npx tauri build`。
Mac 输出 APP 和 DMG；中文名称 DMG 会同时复制到归档目录。
原始 Tauri 产物位于 `src-tauri/target/<Rust目标>/release/bundle/`。
可加 `--debug` 生成包含调试信息的安装包，对应目录为 `debug`；例如 `npm run tauri:build:mac:arm -- --debug --ci`。
可加 `--dry-run` 只检查当前平台并显示命令。

## 系统依赖与架构

- macOS：安装 Xcode Command Line Tools（`xcode-select --install`）。运行最低 macOS 14。使用 rustup 时，Intel 构建需要 `rustup target add x86_64-apple-darwin`，ARM64 需要 `rustup target add aarch64-apple-darwin`；Universal 两个目标都需要。Homebrew Rust 可直接构建其已安装标准库的架构。
- Windows：安装 Visual Studio Build Tools 的 C++ 桌面开发工作负载、Windows SDK、WebView2。x86/ARM64 还需要对应 Rust 标准库和 MSVC 编译工具。x64/x86 输出 NSIS EXE 与 MSI；ARM64 输出 NSIS EXE。
- Linux（Ubuntu/Debian）：安装 `build-essential curl wget file libwebkit2gtk-4.1-dev libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf desktop-file-utils`。输出 DEB、RPM、AppImage。建议在目标架构 Linux 本机或虚拟机中构建；仅添加 Rust target 不会提供交叉编译所需的 GTK/WebKit 系统库。

macOS、Windows、Linux 安装包需要在对应操作系统构建；脚本会拒绝错误系统，不会假装在 Mac 上生成 Windows/Linux 安装包。GitHub Actions 继续负责完整七目标矩阵。
本地打包不需要 GitHub 凭据；发行签名和 macOS 公证按已有 Tauri 环境变量配置。

## 本地虚拟账号回归

`npm run test:system` 启动本地系统（默认 1431 端口）。测试数据不属于线上系统，支持 245 个虚拟用户、部门与工号、分页与增量条件。
原生自动回归使用 1432 端口，步骤见 [verification.md](verification.md)。

```sh
npm run check
npm run build:frontend
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
```
