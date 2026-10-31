# 21: Known Hosts 导入与导出

**What to build:** 用户可通过文件对话框导出和导入信任主机数据，用于换机迁移；导入会校验数据并显式处理重复或冲突。

**Blocked by:** None (can start immediately).

**Status:** merged

- [ ] 可导出完整 known hosts 数据到用户选择的文件。
- [ ] 可导入合法文件，并在设置页看到导入后的主机和指纹。
- [ ] 导入前明确选择冲突处理策略；相同条目和冲突主机密钥不会被静默覆盖。
- [ ] 损坏、不兼容或不可读文件会产生可理解错误，现有数据保持不变。
- [ ] 写入遵循现有原子存储和只读 Recovery 约束，中英文文案完整。

## Progress — 2026-10-25

- 完整实现提交 `8b80f9d` 已串行合入 integration，merge commit/HEAD `086ed3d`。
- 合并后 `cargo check --workspace` 通过，有一个未使用变量警告；未单独运行完整测试。
