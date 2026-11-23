# OxideSSH项目
## 项目概述

这是一个ssh客户端，可以用于管理ssh连接

## 技术栈

- Rust
- GPUI

## 常用命令
- **Build**: `cargo build`
- **Test**: `cargo test`
- **Lint**: `cargo clippy --all-targets`

## 提交与发布

- PR 标题使用 Conventional Commits：`feat`、`fix`、`docs`、`refactor`、`test`、`ci`、`build`、`chore`；可选 scope。破坏性变更在类型或 scope 后加 `!`，或在 footer 写 `BREAKING CHANGE:`。
- 使用 squash merge，保留已校验的 PR 标题作为提交标题；合并前通过 CI。
- 发布版本和 changelog 由 CI 的发布 PR 更新，人工变更中不修改版本。发布规则与审阅流程见 `docs/adr/0006-release-versioning.md`。

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues via `gh`. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default triage labels. See `docs/agents/triage-labels.md`.

### Domain docs

Use the single-context layout. See `docs/agents/domain.md`.
