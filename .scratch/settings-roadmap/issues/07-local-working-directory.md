# 07: 本地终端工作目录

**What to build:** 用户可配置本地终端的启动目录；未配置时继续使用当前 home 目录行为。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 工作目录可配置、持久化，并提供中英文设置文案。
- [ ] 新建和重启本地终端时，Shell 的初始工作目录为配置值。
- [ ] 空配置保留当前 home 目录行为。
- [ ] 目录不存在或不可访问时给出可理解的错误，不静默使用其他目录。
- [ ] 缺字段的旧配置可加载，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 完整实现已提交 `7c20c96`，分支 `ticket/07-local-working-directory` 保留。
- 串行合并后 `cargo check --workspace` 失败：`model.rs:285` 的 `AppSettings` 初始化缺少 `local_working_directory`。
- integration 已回退到 `d2f290b`；后续补齐初始化后重新合并检查。
- 后续已补齐 `AppSettings` 初始化并合入 integration；workspace check、完整 136 项测试及桌面设置页 smoke 通过。
