# 03: 本地 Shell 路径与参数设置

**What to build:** 用户可指定本地终端使用的 Shell 和启动参数，例如 git-bash 或 WSL；未配置时继续使用平台默认 Shell 回退链。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [x] Shell 路径和参数可配置、持久化，并包含中英文设置文案。
- [x] 新建和重启本地终端时使用配置的程序及参数。
- [x] 空配置保留现有默认 Shell 选择行为。
- [x] 程序不存在或启动失败时显示可理解的错误，不静默启动其他 Shell。
- [x] 缺字段的旧配置可加载，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- worktree 保留 10 个文件的未提交实现，尚无 ticket commit、未合入 integration。
- 串行检查发现 `start_local_session` 向 `spawn_local_shell` 传参时引用未定义的 `size`，当前无法通过编译。
- 按规则跳过；后续修正该编译错误并完成剩余验收后重试。

## Progress — 2026-10-31

- 已合入 `integration/settings-roadmap`；最终实现提交 `93f6f48`。
- 代码审查发现的 Unix 测试调用参数和 Shell 参数边界问题已修复。
- workspace check、core/desktop 定向测试及真实本地 Shell 启动路径验证通过。
