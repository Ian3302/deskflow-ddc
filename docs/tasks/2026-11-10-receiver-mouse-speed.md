# 接收端鼠标灵敏度

- 目标：Windows 提供键鼠时，Mac 可独立设置映射鼠标速度，不改变提供端本机手感；支持现有平台组合。
- 分支：`codex/receiver-mouse-speed`
- Worktree：`/Users/yanchengbin/Desktop/DDC-receiver-mouse-speed`
- 目标分支：`origin/main`（基线 `46477a9`）。主工作区已有修改，未复制或改动。

## 计划与进度

- [x] 检查仓库状态、适用规范与固定版本引擎源码。
- [x] 新增持久化设置、接收端 UI、握手传递和提供端引擎参数。
- [x] 验证配置迁移、无效输入、会话参数和界面；运行相关检查。
- [ ] 审查 diff，提交并创建 PR，跟进 CI。

## 实施与关键决策

Deskflow 固定版本 1.27.0 的 `Server::onMouseMoveSecondary` 已实现 `DESKFLOW_MOUSE_ADJUSTMENT` 环境变量：仅缩放接收端鼠标位移，仍由服务端处理边缘和坐标。复用该能力，避免在 Mac 注入层缩放造成点击坐标或跨屏边界不一致。接收端设置通过 Identity 传递，由提供端启动引擎时使用；默认 1×。设置修改遵守现有停止共享、保存后重启流程。

## 验证

- `pnpm install --frozen-lockfile`：已通过；系统 `pnpm` 为 8.15.9，改用 Node 直接运行已缓存的仓库指定 pnpm 11.8.0。未修改依赖或锁文件。
- `pnpm typecheck`：通过。首次新增 UI 测试缺少 Snapshot 字段，已补齐后重跑通过。
- `pnpm test`：78 项通过（17 个文件）。首次 UI 测试夹具缺少 dependencies，已补齐后重跑通过。
- `pnpm build:web`：通过。
- `prettier --check`（全部变更的 Markdown、Vue、TS 文件）与 `git diff --check`：通过。
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib`：首次并发执行 196 通过、1 项既有显示恢复测试超时；按 CI 的 `--test-threads=1` 串行重跑，197 项全部通过（86.33 秒）。
- 未修改原生桥，未在本机执行原生桥测试或完整桌面打包；交由现有多平台 PR CI 验证。
- 无两台实体电脑联调环境，Windows 提供、Mac 接收的实际手感、点击拖拽及跨屏需实机验证。引擎对每个整数位移事件乘倍率后取整，低速单像素移动不会严格按数学倍率缩放。

## PR 与状态

尚未创建；未授权合并。

## 遗留事项

两端需更新到包含本功能的构建。PR 与 CI 待完成；尚未授权合并。
