# 05: 终端 Bell 模式

**What to build:** 用户可选择视觉提示、声音提示或关闭 Bell；远端 Bell 事件只触发所选反馈。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] `visual`、`sound`、`off` 三种模式可配置并持久化。
- [ ] 视觉模式产生可见提示，声音模式实际播放提示音，关闭模式不产生用户反馈。
- [ ] 连续 Bell 和界面重绘不会造成提示卡死或异常重复播放。
- [ ] 缺字段的旧配置保留当前视觉提示行为，`schemaVersion` 保持 1。
- [ ] 中英文设置文案完整。

## Progress — 2026-10-25

- 实现提交：`acc5fb5`、`dd84c6b`；分支 `ticket/05-terminal-bell` 保留。
- 首次集成后 `cargo test --workspace` 失败：`oxide-ssh-core/src/model.rs:519` 存在未闭合分隔符，指向 `terminal_bell_defaults_to_visual_and_round_trips_all_modes`。
- 按串行队列规则跳过，当前未合入 integration。后续先修复合并结构，再运行 workspace 测试及 UI/audio smoke。
- 单独重试时又在 `model.rs`、`app/mod.rs`、设置 UI 和 state 发生冲突，未运行编译；继续保留为后续处理。
- 后续已合并 Bell 模式和视觉提示过期修复；旧配置默认 Visual 的 serde 兼容已补齐，workspace check、完整 136 项测试及桌面设置页 smoke 通过。
