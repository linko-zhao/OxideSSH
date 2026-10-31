# 13: Profile 启动命令

**What to build:** 用户可在 Profile 中配置连接成功后执行的命令，例如 `tmux attach`；未配置时继续使用当前交互 Shell。

**Blocked by:** None (can start immediately).

**Status:** merged

- [ ] 启动命令可在 Profile 创建和编辑流程中填写并持久化。
- [ ] 认证及终端通道建立后，配置命令在实际远端会话中执行。
- [ ] 空配置保留现有交互 Shell 行为。
- [ ] 命令执行失败或远端退出时，会话状态和错误可见，不伪报连接成功后的命令结果。
- [ ] 缺字段的旧 Profile 可加载，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现提交 `4acf922` 已串行合入 integration，最终合并 HEAD `fac1060`。
- 合并后 `cargo check --workspace` 通过；未单独运行完整测试套件。
