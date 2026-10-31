# 22: Profile 级外观覆盖

**What to build:** 用户可为单个 Profile 覆盖终端配色和字号，用外观区分生产与测试连接；未覆盖项继承全局设置。

**Blocked by:** 08: 终端字体、字号、行高与 PTY 尺寸联动

**Status:** done

- [x] Profile 创建和编辑流程可设置、清除配色及字号覆盖。
- [x] 未覆盖时继承当前全局值，显式覆盖时使用 Profile 值。
- [x] 多个不同外观的连接同时打开时，各终端保持独立且重连后正确。
- [x] Profile 字号覆盖后的字符几何和 PTY 行列数保持同步。
- [x] 缺字段的旧 Profile 可加载，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 未开始实现。#08 的首次 PTY 视口尺寸验收尚未完成，因此本票仍被阻塞。

## Progress — 2026-10-31

- 已合入 `integration/settings-roadmap`，最终合并提交 `dbab537`；Profile 配色和字号覆盖、双语文案、独立终端及重连、字符几何与 PTY 尺寸同步均已实现，旧 Profile 继续按 schema v1 加载。
- 两轴代码审查发现并已修复外观校验错误提示及配色解析不一致。
- `cargo test --workspace`：155 passed，1 ignored；`cargo clippy --all-targets` 完成（仅既有警告）；桌面应用成功启动。
