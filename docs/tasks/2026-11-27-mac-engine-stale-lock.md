# 修复 macOS 键鼠引擎残留锁

## 目标与范围

用户希望彻底解决键鼠共享反复因引擎状态码 5 无法重连的问题。现场发现已退出的 Deskflow 引擎留下了 Qt 的 System V 共享内存，macOS 的 32 个共享内存槽位已用满。现有的每次启动独立临时目录可以隔离旧信号量，但无法回收全系统共享的槽位。

## 工作位置

- 任务分支 `codex/mac-engine-stale-lock`
- Worktree `/Users/yanchengbin/Desktop/DDC-mac-engine-stale-lock`
- 目标分支 `main`

## 实施计划

- 清理本次启动创建且已无人使用的 IPC 资源。
- 覆盖父应用突然退出以及引擎退出保护失效时的清理。
- 用 macOS 原生 IPC 验证资源归属和回收条件，运行相关测试。
- 提交 PR，跟进检查并合并。

## 实际修改与决策

- 应用把本次启动的私有临时目录交给退出保护进程。引擎退出后，保护进程用 Qt 的键文件定位共享内存和信号量，并核对系统记录的 UID、引擎 PID 及共享内存附着数再回收。
- 应用在保护进程启动失败或异常结束时，也会在确认引擎退出后尝试回收本次启动的资源。
- 保留每次启动独立临时目录及现有引擎预检查。自动清理限定于本次启动，避免误清历史记录不足或其他程序使用的 IPC。
- 在用户 Mac 上核对日志及内核记录后，共回收 25 个已退出的旧版 Deskflow 引擎槽位，现场占用从 32 降到 7。

- Windows CI 在准备已校验的 Deskflow ZIP 时，PowerShell `Expand-Archive` 因 .NET 方法重载不匹配而失败。Windows 解压改用系统自带的 bsdtar (`tar.exe`)，保持原有哈希校验、临时目录及打包流程。

## 验证

- 本机原生 IPC 单元测试已通过，验证错误 PID 和仍附着的共享内存会被保留，匹配的空闲段与信号量会被回收。
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib -- --test-threads=1`：203 项通过。选用串行验证，避免显示测试的并发计时干扰。
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --test engine_guard -- --test-threads=1`：3 项通过。强制结束父应用后，保护进程终止专属引擎并回收其共享内存，其他进程继续运行。
- `pnpm build:web`、`pnpm test:native`：通过。
- `pnpm test`：85 项通过。官方打包引擎解包、签名以及版本冒烟测试通过。macOS 应用打包、签名校验和压缩包完整性验证通过。
- 实体双机测试待安装修复版后验证。

## PR 与状态

[PR #7](https://github.com/Ian3302/deskflow-ddc/pull/7) 已创建，GitHub Actions 检查待完成。

## 遗留问题

显示输出恢复超时的起因仍需另行复现。
