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

- 提交标题使用 Conventional Commits：`feat`、`fix`、`perf`、`docs`、`refactor`、`test`、`ci`、`build`、`chore`，可选 scope；破坏性变更加 `!` 或 `BREAKING CHANGE:`。
- 按整次推送判定是否发布：含 `feat`、`fix`、`perf` 或破坏性变更才打 tag；只有 `docs`/`ci`/`chore`/`test`/`refactor`/`build` 不打。判定一次、取最高级别，不逐提交打。
- tag 名与版本号一致：`v` + workspace `Cargo.toml` 版本。发布型推送先追加一个 `chore(release): vX.Y.Z` 提交，同时改 `Cargo.toml` 与 `Packager.toml`；破坏性变更在 `0.x` 升 minor（`1.0` 后升 major），`feat` 升 minor，`fix`/`perf` 升 patch。
- 首次发布是基线：首个发布型推送直接打 `v0.1.0`，不 bump。
- 顺序固定：推送 → 等该 SHA 的 `CI` 全绿 → 建 annotated tag（message 汇总上个 tag 以来的 `feat`/`fix`/`perf` 标题）→ `git push origin vX.Y.Z`。等 CI 用 `gh run list --workflow ci.yml --commit <sha>` 找运行，再 `gh run watch <id> --exit-status`；结论不是 `success` 就不打 tag，修好后再推。
- 只给 `origin/main` 顶端打 tag，不给历史提交补打。会话在 CI 结束前结束时，下次先补查该 SHA：绿了补推原 tag，红了放弃。
- 已推送的 tag 不可变：不删除、不移动、不重打同名 tag，发现问题向前发新版本。不新建 `CHANGELOG.md`，发布说明只写在 tag message 里。

## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues via `gh`. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default triage labels. See `docs/agents/triage-labels.md`.

### Domain docs

Use the single-context layout. See `docs/agents/domain.md`.
