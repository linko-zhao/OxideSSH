# 02: SSH Keepalive 默认值与 Profile 覆盖

**What to build:** 用户可配置全局 SSH keepalive 间隔和最大失败次数，并在单个 Profile 中覆盖；连接按最终解析值保活和判定断线。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 全局设置和 Profile 覆盖均可编辑并持久化。
- [ ] Profile 未覆盖时继承当前全局默认值，显式覆盖时优先使用 Profile 值。
- [ ] 新连接和重连实际应用解析后的 keepalive 参数。
- [ ] 缺字段的旧配置保留 30 秒、3 次的现有行为，`schemaVersion` 保持 1。
- [ ] 中英文文案及继承/覆盖行为验证完整。

## Progress — 2026-10-25

- 实现提交：`ebb2ac3`；隔离目标下 Profile 覆盖持久化测试 1 项通过，SSH fixture smoke 未运行。
- 串行合入 #16 时 `model.rs`、`session.rs`、设置 UI、state 及中英文案发生冲突；按规则中止并跳过。
- 分支 `ticket/02-ssh-keepalive` 保留，后续解决集成冲突后重试。
- 后续已解决共享设置冲突并合入 integration；Profile 覆盖、超时和压缩共存，workspace check、SSH session 19 项及完整 136 项测试通过。
