# macOS 显示输出恢复与设置保存修复

## 目标与范围

用户要求修复截图中的保存设置失败。本机日志记录 mac-output restore 提交显示事务返回 1001，错误摘要同时被基线 JSON 占满。修复配对等设置保存对输出恢复的依赖，完善恢复状态确认与重试，并展示真实失败原因。

## 分支与基线

- 分支 `codex/mac-output-recovery`
- worktree `/Users/yanchengbin/Desktop/DDC-mac-output-recovery`
- 目标 `main`，起点为最新 `origin/main` 的 46477a9
- 主工作区已有未提交修改，保留原状。任务独立实现。

## 计划与进度

- [x] 阅读规范、代码和本机诊断日志
- [x] 修复保存和恢复流程并补回归验证
- [x] 改善原生错误摘要
- [x] 前端、原生及硬件验证，审查完整 diff
- [x] 创建 PR
- [x] 完成 Rust 串行复核
- [ ] 跟进 PR CI

## 实际修改与关键决策

- 配对地址、备用 IP 等设置保存独立于输出恢复。旧 guardian 的基线、待恢复标志和 Stop 重试入口持续保留。更改输出 ID、切屏方式、第二屏配置或运行模式仍要求先恢复原输出。
- 原生启用后按 UUID 核验实际 active 状态。事务报告错误时也等待状态落定，并在有界等待后重试一次。目标始终缺失则继续返回真实失败。
- 已启用目标跳过重复启用，位置一致时跳过位置事务。编号冲突校验保留。
- 恢复轮询总等待上限约三秒，配合现有原生调用七秒超时。
- 错误摘要优先提取 stderr，缺失时使用 stdout，原始错误继续保留在详情中。
- 曾验证 CGRestorePermanentDisplayConfiguration 作为恢复兜底，在本机强制注入启用失败后无法重新启用目标，已从最终实现移除。

## 验证

使用仓库指定 pnpm 11.8.0。系统 pnpm 8.15.9 无法读取当前 workspace 配置，默认 npm 缓存存在权限错误，已改用任务 worktree 的忽略缓存目录安装指定版本。依赖和锁文件未变更。

已通过以下检查。

- `pnpm typecheck`
- `pnpm test`，17 个测试文件，79 项测试通过，含新增 4 项错误摘要回归测试
- `pnpm build:web`
- `pnpm test:native`，含 Swift 恢复状态机故障注入测试
- `prettier --check src/shared/error-guidance.ts tests/error-guidance.test.ts`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `git diff --check`

Rust 默认并行测试首次运行 194 项通过，1 项既有测试 accepted_switch_retries_transient_readback_without_undoing_input_or_reissuing_writes 因等待超时失败。新增配对保存回归测试通过。随后执行 `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib -- --test-threads=1`，195 项全部通过，包含该超时项。

本机硬件验证使用任务 worktree 编译的 mac-output。原版 helper 本次恢复成功，历史 1001 未在这次实测重现。最终实现重新捕获基线、停用、恢复均退出 0。恢复后两块外接屏均 active，位置为 (0, 0) 与 (-1920, 0)，符合基线。

Swift 故障注入覆盖已启用目标、错误后立即恢复、错误后延迟恢复、第二次启用成功，以及目标 UUID 始终缺失。Rust 新增回归覆盖 host/client 两种角色的配对保存、恢复基线保留、输出更换受阻和后续 Stop 恢复。

尚未验证双机键鼠转移、肉眼画面和已安装应用的完整入口。桌面打包和跨平台验证交由 CI。

## PR 与状态

PR https://github.com/Ian3302/deskflow-ddc/pull/3 已创建，待合并。用户尚未明确授权合并。实时 CI 结果见 PR 检查页。已运行的应用尚未替换。

## 遗留事项

- 跟进最终提交的 CI。
- 双机键鼠转移和显示器实际画面仍需实测。
