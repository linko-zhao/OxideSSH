# 24: 本地端口转发

**What to build:** 用户可创建、启动、停止和删除 local forwarding 规则，使本地 TCP 监听器通过 SSH 会话转发到指定目标。

**Blocked by:** None (can start immediately).

**Status:** paused

- [ ] 转发规则可配置、持久化，并具有中英文管理界面。
- [ ] 规则启动后，本地 TCP 客户端可通过 SSH 会话访问目标地址和端口。
- [ ] 停止规则会关闭监听器并终止对应转发，不影响同一会话的其他规则。
- [ ] 监听失败、SSH 断连和目标不可达会反映到规则状态，不伪报运行中。
- [ ] 缺字段的旧配置可加载，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 按用户要求暂停；worktree 保留未提交实现，未合并。
- 已落地持久化规则 CRUD、设置管理入口及部分 session-owner listener/channel 生命周期接线。
- 真实 TCP smoke、完整规则状态与测试未确认；#25/#26 仍被本票阻塞。
