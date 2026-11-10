# 双屏 DDC 与第二屏兼容模式修复

## 目标与范围

修复 Mac 切回 Windows 时第二块共享屏幕未切换的问题。用户要求在原接线上支持双向切换、继续修复双端 DDC，并提交、推送及通过 PR 合并。Windows 由用户编译并实测。

## 分支与工作树

- 分支 `codex/dual-display-ddc-031`
- Worktree `/Users/yanchengbin/Desktop/DDC-dual-display-ddc-031`
- 目标 `main`，基线 `origin/main` 的 `2189d96`
- 已有 15 个源码文件的修改由本任务在当前聊天中产生，按明确路径导入任务 worktree。主工作目录新出现的 `AGENTS.md` 和 `docs/tasks/2026-11-10-agent-workflow.md` 属于其他工作，不纳入本 PR。
- 独立工作树规范在此前本地开发完成后出现在工作目录中。本次提交流程遵循该规范，保留已有工作现场。

## 计划与进度

- [x] 排查系统授权、显示器接线与双向切源
- [x] 开发 Mac 第二屏可选自动选源及异常恢复
- [x] 修复 Windows DDC 稳定身份、旧编号映射与失败重试
- [x] 同步构建资源，统一版本为 0.3.1
- [x] 在任务 worktree 中复核验证并提交
- [x] 创建 PR
- [x] 检查 CI
- [x] 用户已授权，通过 GitHub PR 执行合并

## 修改与关键决策

Mac 原 USB-C 转 HDMI 链路发送 DDC 后显示器未切换，直连 HDMI 则成功。经用户实测，暂时停用第二路 Mac 输出会触发显示器自动选择 Windows。新增第二屏可选自动选源模式，复用已有 guardian 保存、恢复显示拓扑，停止共享、连接断开及进程退出均恢复输出。测试操作保留自动恢复期限。

Windows DDC 原来保存 `DISPLAYn#i`。复用现有显示设备接口哈希生成稳定的物理屏幕 ID，保留当前 helper 会话中的旧编号映射，切换前绑定两屏。系统调用失败时最多尝试三次，并重新枚举句柄，正常成功路径不添加重试等待。

Windows 构建时同步源目录的 PowerShell 控制桥，避免打包过期生成文件。增加已有 Windows 双屏配置的解码回归测试。版本统一为 0.3.1，区分不支持 `secondDisplay` 的旧 0.3.0 构建。

## 验证

此前主目录验证已通过前端 75 项、Rust 库 193 项（之后新增的配置测试单独通过），原生 C# 绑定在 Mono 上编译并执行离线身份与重试测试，Mac 应用打包成功。用户已确认兼容模式下两屏 Mac → Windows、Windows → Mac 都成功。Mac 0.3.1 已安装，界面显示版本正确且辅助功能已授权。

任务 worktree 中已通过下列验证。

- `pnpm install --frozen-lockfile`
- `pnpm typecheck`
- `pnpm test`，75 项通过
- `pnpm build:web`
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib -- --test-threads=1`，194 项通过
- `node scripts/test-native.mjs`，macOS 原生离线测试通过
- 从当前 Windows PowerShell 桥提取原生 C#，用 `csc` 编译 `native/tests/windows-screen-identity.cs` 并用 `mono` 执行，稳定 ID、旧编号重排、失败重试与 ABI 测试通过，未调用 Windows API
- `prettier --check` 针对修改的前端、配置和任务文档通过
- `git diff --check` 通过

Windows 实际 API、安装包和 macOS 桌面全特性检查交由 PR CI。原生硬件 DDC 验证由用户在 Windows 更新后完成。

## PR 与状态

PR https://github.com/Ian3302/deskflow-ddc/pull/1 已创建并推送。用户已授权合并，四个平台检查均成功，按 GitHub PR 合并交付。

- Windows x64 与 macOS ARM64 测试和安装包构建成功 https://github.com/Ian3302/deskflow-ddc/actions/runs/42752594763
- Ubuntu x64 与 ARM64 测试和 deb 构建成功 https://github.com/Ian3302/deskflow-ddc/actions/runs/42752598366
- 上述检查对应代码提交 `96b486281e90b5d74221028b5fbdac6f8f075814`。最终提交仅补齐本任务文档，不改变已验证代码。

## 遗留问题

纯 DDC 在用户 Windows 显示硬件上的效果尚待编译后验证。离线身份测试不能证明转接器或非活动 DP 通道支持物理 DDC。Windows 用户需重新扫描并保存两个物理屏幕的映射。两端应用版本必须一致。
