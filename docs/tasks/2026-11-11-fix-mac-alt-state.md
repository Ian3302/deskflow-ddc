# Windows → Mac Alt 状态异常排查与修复

## 目标与范围

用户反馈 Windows 键鼠控制 Mac 时，按 Alt 后字母变为特殊符号，需要反复按 Alt 才能恢复；已确认 Mac 粘滞键关闭。目标是定位并修复修饰键状态残留。不能把未复现的推测写成已解决。

## 工作区

- 分支：`codex/fix-mac-alt-state`
- worktree：`/Users/yanchengbin/Desktop/DDC-fix-mac-alt-state`
- 目标分支：`main`；基线 `origin/main` 的 `88a714c`
- 主目录及其他任务 worktree 的已有修改保持不动。

## 计划与进度

- [x] 阅读仓库规范、输入共享配置、固定版本引擎源码及既有排查记录。
- [x] 确认应用没有单按 Alt 循环切换映射的功能。
- [x] 检查并修正原生切换快捷键的修饰键释放。
- [x] 增加只包含修饰键状态的诊断；异常现场尚未捕获。
- [x] 执行原生回归检查，审查完整 diff。
- [x] 推送分支并建立草稿 PR。

## 已知事实与关键决策

应用映射只交换 Ctrl 与 Command。内置 Deskflow 1.27.0 的 macOS 接收端自行维护修饰键状态；目前日志没有逐键收发记录，尚未确认用户现象的具体触发路径。

原生 macOS 内部切换快捷键为 F18/F19 附带 Ctrl+Option+Shift，现有按下和松开事件均携带这组三个修饰键，且没有恢复注入前的标志。修复此不平衡序列时必须保留原本按住的按键，禁止盲目清空所有修饰键或定时打断合法长按。

## 验证

- `node scripts/test-native.mjs`：通过，包括 DDC、Mac 显示输出、原生生命周期、快捷键事件检查、诊断结构和有界状态观察器参数验证。
- 快捷键回归在不发送系统输入的前提下检查真实 CGEvent：F18/F19 的事件类型与键码、松开后恢复空闲或原已按住的修饰键/Caps Lock 状态，以及按下事件保持精确内部组合以免 Command 等额外标志使绑定失效。
- `node scripts/build-native.mjs`：通过，按生产目标 macOS 14 arm64 编译原生 helper。
- `git diff --check`：通过。
- 无前端、Rust 或依赖变更，未在本地重复执行前端/Rust 全量测试；PR CI 状态另行记录。
- 创建 PR 后查询 `gh pr checks 5` 返回 no checks reported；`gh run list --branch codex/fix-mac-alt-state` 返回空列表，本次没有自动触发 CI，因此不记录为 CI 通过。
- 用户表示当前不方便复现，已结束临时只读观察，未进行输入注入或安装/重启现有应用。双机故障现场仍待捕获；初始快照的各修饰键均为松开，不能据此排除间歇性故障。

## 实际修改

- 内部 F18/F19 快捷键的松开事件恢复注入前的 session 标志，不再无条件继续携带人工 Ctrl+Option+Shift，也不盲目清空用户原先按住的修饰键。
- `diagnose` 增加 HID 与 combined session 的左右 Control/Option/Shift/Command 状态及聚合标志。
- 提供 `watch-modifiers 1..60` 的有界只读轮询，20ms 采样，仅在状态变化时输出时间和修饰键，不读取普通键或输入内容。轮询无法保证捕捉短于采样间隔的变化。
- 观察命令使用构建产物 `resources/bin/mac-hotkey watch-modifiers 60`；它不是自动恢复机制，不会定时释放按键。

## PR 与状态

草稿 PR：https://github.com/Ian3302/deskflow-ddc/pull/5 。代码提交 `c50cfa3`。未合并，用户尚未授权合并；由于尚未证明解决用户的物理 Alt 故障，保持 draft。

## 遗留事项

内部快捷键修复不等于已确认解决所有物理 Alt 输入问题，必须区分自动回归结果与用户双机验证。下一步需要故障出现时的修饰键快照，以及是否在屏幕切换后、左 Alt 或右 Alt 下触发的信息；如物理 Alt 仍可独立触发，需要继续检查两端引擎的收发与 macOS HID 注入。
