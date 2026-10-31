# 11: OSC 52 剪贴板三态策略

**What to build:** 用户可选择 `off`、`ask` 或 `on`，控制远端应用通过 OSC 52 写系统剪贴板；询问模式在明确授权前不写入。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 三态策略可配置、持久化，并提供中英文设置及确认文案。
- [ ] `off` 不触发系统剪贴板写入，`on` 写入远端提供的完整内容。
- [ ] `ask` 显示确认界面，仅授权后写入；拒绝、取消和关闭均不修改剪贴板。
- [ ] 多个连续 OSC 52 请求不会造成绕过确认或重复写入。
- [ ] 缺字段的旧配置默认 `off`，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 完整实现位于提交 `e122130`，分支 `ticket/11-osc52-clipboard` 保留。
- 串行合并后 workspace check 检出 `default_confirm_before_close_with_sessions` 重复定义，以及 `ssh_compression` 字段和初始化重复。
- 已用 merge+revert 恢复 integration 内容（当前历史 HEAD `dadc935`），本票未生效；后续去除重复定义后重试。
- 后续已去重并串行合入 OSC 52 策略、授权复核和设置 UI；旧配置默认 Off 已补齐，workspace check、完整 136 项测试及桌面设置页 smoke 通过。
