# 29: 配置导入、导出与备份恢复

**What to build:** 用户可导出、导入和备份恢复应用配置，用于迁移 Profiles、标签及设置；导出文件不包含 OS keyring 中的秘密。

**Blocked by:** None (can start immediately).

**Status:** completed

- [x] 可将当前配置导出到用户选择的文件，界面明确声明不包含密码、私钥等 keyring 秘密。
- [x] 可创建备份并从合法导出或备份文件恢复，重启后配置内容一致。
- [x] 导入前校验格式、版本和字段，损坏或不兼容文件不会覆盖现有配置。
- [x] 导入后缺失的凭据会要求重新录入，不伪造可用认证信息。
- [x] 存储写入保持原子性和只读 Recovery 约束，中英文文案完整。

## Progress — 2026-10-31

- 已完成并合并至 `integration/settings-roadmap`（合并提交 `26e4ed1`）。
- core storage 测试 22 项通过；desktop check 与中英文覆盖测试通过。
- 代码审查发现的后台 I/O、严格字段校验及恢复后控件同步问题均已修复。
