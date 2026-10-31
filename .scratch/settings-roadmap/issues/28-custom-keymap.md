# 28: 快捷键自定义与冲突检测

**What to build:** 用户可修改应用快捷键并持久化；保存前发现冲突，实际键盘分派使用新的映射。

**Blocked by:** None (can start immediately).

**Status:** paused

- [ ] 设置界面可查看和编辑支持自定义的动作快捷键，中英文文案完整。
- [ ] 同一上下文中的冲突在保存前被指出，不产生不确定分派。
- [ ] 保存后各实际界面使用新快捷键，应用重启后仍生效。
- [ ] 用户可恢复默认映射，非法或无法表示的按键不会破坏配置。
- [ ] 缺字段的旧配置保留当前快捷键，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 按用户要求暂停；worktree 保留未提交实现，未合并。
- 已实现 keymap schema/冲突检测、capture/恢复默认 UI、中英文文案、runtime bind/reset 和 README；core 39 项测试此前通过。
- desktop 编译和实际快捷键 smoke 未完成，后续先验证当前现场。
