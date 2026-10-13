# 兰谷多账号工具工程约定

## 技术栈与目录

- 前端使用 Vue 3、TypeScript、Pinia、Vue Router；构建使用 Vite 和 pnpm。
- 桌面层使用 Tauri 2 / Rust，按 commands、services、domain、infra 分层。
- 页面放 src/views/<module>，公共组件放 src/components/<module>，状态放 src/stores。
- 页面不得直接调用系统接口；经 src/api 调用 Rust command。
- 同级 langu-web-oa 是接入协议参考，不属于此工程，不能将其 Vue 2 规范套用到本工程。

## 会话与权限

- 每个账号必须拥有独立 WebView/profile。不能通过共享 iframe 或覆盖主账号缓存实现切换。
- 只能用主账号会话访问检索和代登录接口，不能从当前选中的子账号提取管理凭据。
- token、完整登录响应不得写入前端状态、事件、日志、仓库、URL 或 settings.json。
- 所有自定义 command 必须校验调用来源。远程 WebView 不获得客户端权限。
- 注入和读取登录缓存必须校验完整 origin 和顶层页面。
- 权限由服务端执行；不得绕过服务端鉴权、权限、验证码和证书校验。
- 账号数不设置业务上限；新增资源管理机制时不得悄悄丢弃用户页面状态。

## 开发与验证

- 手工改动使用小范围修改；保留用户现有改动。
- 命名：Vue 组件 PascalCase，TS 变量 camelCase，Rust 模块 snake_case。
- pnpm-lock.yaml、Cargo.lock 必须入库；忽略 node_modules、target、dist、work。
- 提交前运行 pnpm check、pnpm build:frontend、cargo fmt --check、cargo test。
- 改动账号协议或隔离机制时运行本地测试系统并验证主/子账号不串号。
- Conventional Commits：feat/fix/docs/refactor/test/chore(scope): 描述。
