# 27: Agent Forwarding Profile 开关

**What to build:** 用户可在单个 Profile 中启用 agent forwarding，使远端会话通过转发 channel 使用本地 SSH agent；默认关闭。

**Blocked by:** None (can start immediately).

**Status:** paused

- [ ] Profile 创建和编辑流程可配置该开关并持久化。
- [ ] 开启后，远端进程可通过转发的 agent channel 完成实际 agent 操作。
- [ ] 关闭时不建立 agent 转发，远端无法经本功能访问本地 agent。
- [ ] agent 不可用或转发失败时，错误和会话状态真实可见。
- [ ] 缺字段的旧 Profile 默认关闭，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 按用户要求暂停；worktree 保留 16 个文件的未提交实现，未合并。
- 已接通 Profile 开关、会话 agent channel/原始流代理及桌面状态/i18n；最终测试在运行中被暂停，结果未确认。
- 后续先运行 core/desktop 测试和真实 agent 操作 smoke。
