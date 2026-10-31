# 10: 光标样式、闪烁与远端控制

**What to build:** 用户可配置默认光标形状和闪烁行为，终端同时正确处理远端光标变化事件，并采用一致的优先级。

**Blocked by:** None (can start immediately).

**Status:** complete

- [x] 光标形状和闪烁设置可编辑、持久化，并提供中英文文案。
- [x] renderer 按配置显示默认光标，不再忽略远端光标闪烁事件。
- [x] 远端应用改变形状或闪烁后，显示行为符合定义的远端/本地优先级。
- [x] 焦点丢失、重新聚焦和终端重置不会产生错误光标状态。
- [x] 缺字段的旧配置保持兼容，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现提交 `f2628b3`；core+terminal 53 项测试和 isolated desktop check 通过，worktree 另有后续未提交改动。
- 尚缺真实 timed blink 动画、焦点丢失/恢复及终端重置的完整状态处理，未运行 GUI smoke。
- 按规则不合并，保留 `ticket/10-cursor-style-blink` 后续完成。

## Progress — 2026-10-31

- 已合并到 `integration/settings-roadmap`；修复提交 `748fff7`、`831e64f`。
- core+terminal 90 项测试、desktop 37 项测试通过；clippy 完成，仅有既有警告。
- 代码审查发现并已修复隐藏光标、同形状 DECSCUSR、同块重置后命令的优先级问题。
- GUI 启动受 crates.io 网络与离线缓存缺少 `portable-pty` 阻塞，未完成视觉 smoke。
