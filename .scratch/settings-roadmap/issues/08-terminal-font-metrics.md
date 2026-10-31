# 08: 终端字体、字号、行高与 PTY 尺寸联动

**What to build:** 用户可配置终端字体、字号和行高；显示布局、renderer 字符几何与初始 PTY 行列数保持一致。

**Blocked by:** None (can start immediately).

**Status:** done

- [x] 三项设置均可编辑、持久化，并提供中英文文案。
- [x] 新终端使用配置渲染文本，字符单元格尺寸来自实际字体几何。
- [x] 首次连接时 PTY 行列数与可见终端匹配，不再依赖固定字号或近似系数产生错误尺寸。
- [x] 设置变化和窗口 resize 后的终端布局及 PTY 尺寸保持同步。
- [x] 缺字段的旧配置保留现有字体、13px 字号和 17px 行高行为，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 分支 `ticket/08-terminal-font-metrics` 保留，HEAD `a89a898`；字体设置、双语文案、实际字体几何和 resize 测试已实现。
- 串行检查确认 `estimated_terminal_size` 仍用固定 chrome 估算可见区域，首次 PTY 尺寸不能保证匹配真实布局，验收未完成。
- 未合入 integration；后续修正首次视口尺寸计算后重试。#22 仍被本票阻塞。

## Progress — 2026-10-31

- 已合入 `integration/settings-roadmap`，最终提交 `e74d0b9`；每个新 SSH/本地终端等待自身可见视口的实际几何后启动，设置与 resize 继续同步 PTY。
- `cargo test --workspace`：140 passed，1 ignored；`cargo clippy --all-targets` 完成（仅既有警告）；桌面应用成功启动。#22 已不再被本票阻塞。
