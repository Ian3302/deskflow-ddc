# macOS 显示输出恢复修复

## 目标与范围

用户要求修复截图中的保存设置失败。日志显示 mac-output restore 提交显示事务返回 1001，并且错误摘要被基线 JSON 占满。处理输出恢复失败的兜底、重复执行和真实错误展示，保留失败时的恢复状态。

## 分支与基线

- 分支 `codex/mac-output-recovery`
- worktree `/Users/yanchengbin/Desktop/DDC-mac-output-recovery`
- 目标 `main`，起点为最新 `origin/main` 的 46477a9
- 主工作区已有未提交修改，保留原状。任务独立实现。

## 计划与进度

- [x] 阅读规范、代码和本机诊断日志
- [x] 修复恢复流程并补回归验证
- [x] 改善原生错误摘要
- [x] 本地验证和完整 diff 审查
- [ ] 提交并创建 PR，跟进 CI

## 实际修改与关键决策

- 原生启用失败或启用后 UUID 未恢复时，调用 `CGRestorePermanentDisplayConfiguration` 恢复用户保存的 macOS 显示配置，再按 UUID 确认目标已启用，随后恢复捕获的屏幕位置。
- 该系统兜底会恢复所有显示器的永久配置，可能恢复会话期间调整的模式或镜像设置。仅在原生恢复失败且目标仍未启用时调用。未确认恢复时继续返回失败，现有 guardian 会保留恢复基线。
- 已启用的目标跳过重复启用，位置一致时跳过位置事务。编号冲突检查在兜底之前执行。
- 原生恢复状态机等待上限约四秒，配合现有原生调用七秒超时。
- 错误摘要优先提取 stderr，缺失时使用 stdout，完整原始错误继续保留在详情中。
- 苹果文档说明该系统接口用于撤销临时显示配置变更，参考 https://developer.apple.com/documentation/coregraphics/cgrestorepermanentdisplayconfiguration()

## 验证

使用仓库指定 pnpm 11.8.0。系统 pnpm 8.15.9 无法读取当前 workspace 配置，默认 npm 缓存也存在权限错误，已改用任务 worktree 的忽略缓存目录安装指定版本，依赖和锁文件未变更。

已通过以下检查。

- `pnpm typecheck`
- `pnpm test`，17 个测试文件，79 项测试通过，含新增的 4 项错误摘要回归测试
- `pnpm build:web`
- `pnpm test:native`，含 Swift 恢复状态机故障注入测试
- `prettier --check src/shared/error-guidance.ts tests/error-guidance.test.ts`
- `git diff --check`

本机硬件验证使用任务 worktree 编译的 mac-output 和诊断日志中的原始基线。验证开始时目标输出未出现在在线列表，原版 helper 本次恢复成功，历史 1001 未在这次实测重现。随后用修复版执行一次重新捕获、停用、恢复，操作均退出 0。停用后目标 UUID 消失，恢复后两块外接屏均 active，位置为 (0, 0) 与 (-1920, 0)，符合基线。没有验证双机键鼠转移或肉眼画面。

1001 分支通过无原生调用的故障注入验证，覆盖原生失败后系统兜底成功、无确认后兜底、部分成功和目标一直缺失的有界失败。目标仍缺失时保留原始 1001 错误。

未执行 `pnpm test:rust`，本次未修改 Rust。完整桌面打包与跨平台测试交由 PR CI。

## PR 与状态

尚未创建。用户尚未明确授权合并。已运行的应用尚未替换。

## 遗留事项

- 跟进 PR CI。
- 双机键鼠转移和显示器实际画面仍需实测。
