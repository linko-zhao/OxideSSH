# 14: 窗口尺寸记忆与恢复

**What to build:** 用户可开启窗口尺寸记忆；应用保存窗口尺寸并在下次启动时恢复到可用显示区域。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 开关可配置、持久化，并提供中英文文案。
- [ ] 开启时保存实际窗口尺寸，下次启动按保存值创建窗口。
- [ ] 关闭时不再恢复旧尺寸，也不继续使用过期尺寸数据。
- [ ] 显示器移除或分辨率变小时，窗口被约束到当前可用显示区域。
- [ ] 缺字段的旧配置保持当前启动行为，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现已在提交 `465f3ee` 及后续修复中，分支 `ticket/14-window-size-memory` 保留。
- 串行合并时 `model.rs`、设置 UI 和 i18n 冲突；按规则中止，未运行 workspace check。
- integration 保持 `fac1060`；后续与当前设置基线协调后重试。
- 后续已与当前设置基线串行合入；恢复尺寸按实际 display bounds 约束，workspace check、完整 136 项测试及桌面设置页 smoke 通过。
