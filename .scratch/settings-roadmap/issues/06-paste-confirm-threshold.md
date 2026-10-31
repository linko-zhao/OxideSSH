# 06: 大段粘贴确认阈值

**What to build:** 用户可配置粘贴确认阈值；超过阈值的文本在发送前需要确认，取消时不向会话写入任何内容。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 阈值可配置、持久化，并提供中英文设置及确认文案。
- [ ] 阈值以下直接粘贴，超过阈值时确认后才发送完整原文。
- [ ] 取消或关闭确认框不会发送部分或全部文本。
- [ ] 现有 1 MiB 安全上限及超限错误仍然有效。
- [ ] 缺字段的旧配置保持兼容，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 已提交实现 `af2b352`，包含阈值持久化、中英文确认 UI、完整原文发送及 1 MiB 上限。
- 串行合并时 `model.rs` 和设置 UI 冲突；按规则中止，未运行 workspace check。
- integration 保持 `d2f290b`，分支 `ticket/06-paste-confirm-threshold` 保留，后续解决冲突后重试。
- 后续已解决共享设置和确认动作冲突并合入 integration；workspace check、完整 136 项测试及桌面设置页 smoke 通过。
