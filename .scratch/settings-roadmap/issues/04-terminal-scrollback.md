# 04: 终端 Scrollback 行数设置

**What to build:** 用户可调整终端回滚容量，新终端按配置保留历史输出。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] Scrollback 行数可编辑、持久化，并提供中英文文案。
- [ ] 新终端的实际回滚容量符合配置，超出容量后按预期丢弃最旧内容。
- [ ] 非法值无法保存，缺字段的旧配置保留 10,000 行行为。
- [ ] 已打开终端的行为有明确且一致的定义，不因设置刷新破坏终端状态。
- [ ] `schemaVersion` 保持 1，旧配置加载不进入 Recovery。

## Progress — 2026-10-25

- 已提交实现 `58a4574`，分支 `ticket/04-terminal-scrollback` 保留。
- 串行合并解决了冲突，但 `cargo check --workspace` 检出 `oxide-ssh-core/src/model.rs` 未闭合分隔符。
- integration 已回退到 `d2f290b`，本票跳过；后续修复合并语法问题后重试。
- 后续已串行合入 integration；workspace check、scrollback 目标测试、完整 136 项测试及桌面设置页 smoke 通过。
