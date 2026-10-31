# 23: ProxyJump 跳板机连接

**What to build:** 用户可在 Profile 中配置跳板机，经由跳板机建立目标主机的认证、主机密钥校验和交互终端会话。

**Blocked by:** None (can start immediately).

**Status:** paused

- [ ] Profile 创建和编辑流程可选择或配置跳板机并持久化。
- [ ] 连接依次建立跳板机和目标主机通道，最终终端与目标主机交互。
- [ ] 跳板机和目标主机分别执行正确的认证及主机密钥信任校验。
- [ ] 任一跳失败时显示对应阶段的错误，不伪报目标主机已连接。
- [ ] 缺字段的旧 Profile 保持直接连接行为，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 按用户要求暂停功能级重任务；worktree 保留 14 个文件的未提交改动，未合并。
- 已实现 Profile 跳板选择/持久化、双跳 direct-tcpip/connect_stream、独立认证/host key 和阶段错误；仅兼容性测试 1 项通过。
- 完整双跳测试、desktop/storage 测试和 workspace check 未完成，后续恢复时先验证当前实现。
