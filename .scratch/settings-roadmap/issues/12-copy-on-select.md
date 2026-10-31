# 12: 选中即复制

**What to build:** 用户可开启选中即复制，使鼠标完成有效文本选择后自动写入系统剪贴板。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 设置可编辑、持久化，并提供中英文文案。
- [ ] 开启后，有效鼠标选区完成时复制其完整文本。
- [ ] 空选区、普通点击和仅移动鼠标不修改剪贴板。
- [ ] 关闭设置时保留现有显式复制行为。
- [ ] 缺字段的旧配置默认关闭，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现提交：`373ef02`；分支 `ticket/12-copy-on-select` 保留。
- 串行合入 #16+#09 时 `model.rs`、设置 UI、i18n 和 state 发生冲突；按规则中止，未运行编译。
- 后续解决共享设置冲突后重新合并并验证鼠标选区行为。
- 后续已解决共享设置冲突并合入 integration；workspace check、完整 136 项测试及桌面设置页 smoke 通过。
