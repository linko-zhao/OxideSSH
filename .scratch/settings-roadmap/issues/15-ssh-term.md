# 15: SSH TERM 类型设置

**What to build:** 用户可配置远端 PTY 的 TERM 类型，以适配 tmux 或特殊远端应用；默认仍为 `xterm-256color`。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] TERM 可编辑、持久化，并提供中英文设置文案。
- [ ] 新连接和重连请求的远端 PTY 实际使用配置值。
- [ ] 空值或非法值无法导致无 TERM 的意外会话。
- [ ] 缺字段的旧配置使用 `xterm-256color`，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现提交 `626f077`；desktop check、设置持久化、默认值/非法值测试通过，PTY fixture 确认服务端收到 `tmux-256color`。
- 串行合入 #16+#09 时 core model/session、设置 UI 等 7 个文件冲突；按规则中止，未运行集成编译。
- 分支 `ticket/15-ssh-term` 保留，后续解决冲突后重试。
- 后续已解决共享会话和设置冲突并合入 integration；workspace check、SSH session 19 项及完整 136 项测试通过，桌面设置页 smoke 确认 TERM 控件可见。
