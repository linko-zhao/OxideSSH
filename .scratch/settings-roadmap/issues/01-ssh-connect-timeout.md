# 01: SSH 连接超时设置

**What to build:** 用户可在设置中配置 SSH 连接超时时间，新连接和重连在弱网或高延迟环境下均按该值判定超时。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 设置可编辑、持久化，并在应用重启后恢复。
- [ ] 新 SSH 连接和重连实际使用配置的超时时间。
- [ ] 非法值无法保存，缺字段的旧配置使用现有 15 秒行为。
- [ ] `schemaVersion` 保持 1，旧配置加载不进入 Recovery。
- [ ] 中英文设置文案完整。

## Progress — 2026-10-25

- 实现提交：`d95cff4`；分支 `ticket/01-ssh-connect-timeout` 保留。
- 串行合入 #16 后 `cargo check --workspace` 失败：`model.rs:287` 的 `AppSettings` 初始化缺少 `ssh_compression`。
- 已按规则回退该次合入并跳过；后续补齐集成初始化后重新合并和测试。
- 后续已重新串行合入 integration；workspace check、完整 136 项测试及桌面设置页 smoke 通过。
