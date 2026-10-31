# 18: 自定义 ANSI 16 色调色板

**What to build:** 用户可编辑并持久化终端 ANSI 16 色，终端中的标准和高亮色输出按自定义调色板显示。

**Blocked by:** None (can start immediately).

**Status:** merged

- [ ] 16 色均可通过设置界面编辑、重置并持久化，中英文文案完整。
- [ ] 标准色和高亮色在真实终端输出中使用配置颜色。
- [ ] 主题切换与自定义调色板之间的优先级一致且可预测。
- [ ] 非法颜色值无法破坏配置或终端渲染。
- [ ] 缺字段的旧配置保留当前固定调色板，`schemaVersion` 保持 1。

## Progress — 2026-10-25

- 实现提交 `0580888` 已串行合入 integration，最终 HEAD `a18dd31`。
- 合并后 `cargo check --workspace` 通过，存在一个未使用变量 `cx` 警告；未运行完整测试套件。
