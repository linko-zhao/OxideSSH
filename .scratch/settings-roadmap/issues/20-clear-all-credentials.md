# 20: 清空本应用全部凭据

**What to build:** 用户可在设置页确认后删除 OS keyring 中所有属于本应用的凭据，包括已无 Profile 引用的遗留项，并看到真实的成功或失败结果。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 清空操作有明确确认，中英文说明覆盖删除范围和不可逆性。
- [ ] 能发现并删除本应用凭据，不仅限于当前配置仍可引用的条目。
- [ ] 不删除其他应用的 keyring 条目。
- [ ] 全部删除后，相关 Profile 不再能读取旧秘密；部分失败会列出失败结果，不伪报全部成功。
- [ ] 配置和 Profile 数据在凭据删除过程中不被意外清除。

## Progress — 2026-10-25

- 实现提交 `ac17db9` 已串行合入 integration，最终 HEAD `e727f3d`。
- 合并后 `cargo check --workspace` 通过，存在一个未使用变量警告；未运行完整测试或真实 OS keyring smoke。
- 最终 `cargo test --workspace` 失败：`credentials.rs:792` 对 `Option<SecretBox<str>>` 使用 `assert_eq!`，但 `SecretBox<str>` 未实现 `PartialEq`。按规则不修复，转为后续处理并回退合入。
- 尝试回退 #20 时与后续设置 UI 改动冲突，按规则未修复；因此 #20 仍留在 integration `086ed3d`，但 workspace 测试目标当前不可编译。
- 后续删除重复且不兼容的 secret 断言，目标回归测试及完整 136 项测试通过；未运行真实 OS keyring smoke。
