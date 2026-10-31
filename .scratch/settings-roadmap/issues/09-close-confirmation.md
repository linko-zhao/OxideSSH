# 09: 活动会话关闭确认开关

**What to build:** 用户可通过一个设置控制现有活动会话关闭确认，覆盖标签页、窗口和应用退出路径。

**Blocked by:** None (can start immediately).

**Status:** merged

- [ ] 设置可编辑、持久化，并提供中英文文案。
- [ ] 开启时，关闭有活动会话的标签页、窗口或应用均需确认。
- [ ] 关闭设置时，上述路径不显示活动会话确认。
- [ ] 无活动会话时不因该功能引入多余确认。
- [ ] 缺字段的旧配置保留当前确认行为，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现提交 `d0ba9a3` 已串行合入 integration，合并提交 `d2f290b`。
- 合并后 `cargo check --workspace` 通过；未单独运行 GUI 关闭路径 smoke。
