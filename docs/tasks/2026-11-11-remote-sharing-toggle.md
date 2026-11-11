# 单端控制共享启停

## 目标与范围

工作台右上角直接开启/关闭共享。一台电脑操作，另一台按已保存配置响应，不需双方重复点击。首次配对和系统权限仍需完成；两端 App 需运行且电脑唤醒。

## 工作分支

- 分支：`codex/remote-sharing-toggle`
- Worktree：`/Users/yanchengbin/Desktop/DDC-remote-sharing-toggle`
- 目标：`main`，从已获取的 `origin/main` 创建。
- 主目录已有修改未纳入本任务。

## 计划与进度

- [x] 检查共享生命周期、配对身份与权限检查。
- [x] 添加独立加密待命通道和待命设备发现。
- [x] 接入单端启停与右上角按钮。
- [x] 添加配置与协议测试，完成本地验证。
- [x] 创建 PR，并检查 GitHub 检查状态。
- [ ] 用户已明确授权合并：补充持续授权与完成标准后，执行 GitHub PR 合并并核实远端。

## 修改及关键决策

- 右上角未运行时显示开启按钮，未配对时进入设备页。
- 新增 `allowRemoteStart`，默认允许已配对电脑开启，可在共享设置关闭。
- 独立待命监听只在联网共享且已有保存配对时启用。使用现有加密握手，双方核验 UUID 和证书指纹；不能借待命请求首次配对或替换身份。
- 独立 mDNS 服务支持两种角色，停止共享后仍可发现对端，不改变原先首次配对发现列表。
- 待命端口为控制端口加 1（65535 时减 1），校验禁止与键鼠端口冲突。
- 开启调用原有启动流程，保留配置、系统权限、引擎等检查；对端启动失败时停止本机并发送清理停止请求。
- 停止先执行本机恢复，再通知对端；对端先确认接收请求再执行紧急停止，避免双端同时停止时互等操作锁。通知失败记活动记录；对端恢复失败记录在对端。
- 本地停止不会向对端再次回传停止，避免循环请求。待命通道独立于共享连接，停止后继续可用。
- 更新 README 的使用前提及防火墙端口说明。

## 验证

- `pnpm` 系统版本为 8，与仓库要求的 11 不符；通过临时 npm 缓存运行 `pnpm@11.8.0 install --frozen-lockfile` 安装，未修改锁文件。
- `vue-tsc --noEmit`、`vue-tsc -p tsconfig.node.json --noEmit`：通过。
- `vitest run`：18 个文件、85 项测试通过。
- `vite build`：通过。
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib`：第一次并行执行 200 通过、1 项既有显示恢复测试超时；该测试独立复测通过。
- 同一全套测试加 `-- --test-threads=1`：201 项通过。
- 随后补充/完善协议测试，`cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --lib runtime::standby -- --test-threads=1`：最新 3 项通过，覆盖身份与配置拒绝、普通启动依赖检查、操作锁占用时仍能确认停止、共享停止后待命通道仍可访问。
- `node scripts/prepare-engine.mjs`：通过，准备已校验引擎并构建本机原生桥。
- `cargo check --locked --manifest-path src-tauri/Cargo.toml`：完整桌面特性编译检查通过。首次因 worktree 尚未准备资源失败，准备后通过。
- Prettier 检查、`git diff --check`：通过。
- 未执行真实 Mac/Windows 双机验收；本任务未修改原生桥，未额外运行 native 测试。跨平台打包交由 PR CI。

## PR 与交付

PR：https://github.com/Ian3302/deskflow-ddc/pull/6 ，用户已明确要求合并；本次补充规则后执行合并并核实远端，实时状态以 PR 链接为准。

创建后 GitHub 未报告 PR 自动检查，已手动在任务分支触发仓库既有工作流，并跟进至全部完成：

- [Tauri Build](https://github.com/Ian3302/deskflow-ddc/actions/runs/42981633739)：SUCCESS，macOS ARM64 和 Windows x64 的前端、原生、完整 Rust、引擎集成测试与安装包构建均通过。
- [Ubuntu experimental build](https://github.com/Ian3302/deskflow-ddc/actions/runs/42981638448)：SUCCESS，x64 与 ARM64 的测试及安装包构建均通过。
- CI 验证提交：`dc1c543a9b2a1951c2c1715e871b3cc29feb59c5`；之后仅补充任务记录与 AGENTS.md 规则，不修改功能代码。
- 安装包已上传对应运行的 artifacts（仓库配置保留 7 天）。

## 遗留事项

- 两端均需更新到支持独立待命协议的版本；本次尚未替换用户已安装 App。
- 真实双机验证需确认两端共享配置与系统授权，以及局域网待命 TCP 端口可达。
- 停止响应表示对端已接收请求，实际输出恢复结果以对端记录和显示器为准。

## 用户补充要求：合并才算完成

用户明确要求“写好规则，只有合并才算任务结束”。已将完整代理规范纳入本任务分支，补充持续授权：检查、评审和分支保护满足后主动合并，无需重复询问；只有 GitHub PR 已合并且远端目标分支包含修改才算完成。主工作目录的 AGENTS.md 同步更新此规则，其他既有修改不动。阻碍合并时必须明确任务未完成，不能用“待合并 PR”作为已完成交付。

本次仅变更规则与任务记录，复用已通过的四平台功能验证，不重复运行代码测试；提交前执行差异与 Markdown 格式检查。
