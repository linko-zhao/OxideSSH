# 使用发布 PR 管理应用版本

OxideSSH 使用 release-plz 的 Git-only 模式维护单一候选发布 PR。普通 PR 标题采用 Conventional Commits，并通过 squash merge 进入 `main`；`feat` 升 minor（包括 `0.x`），`fix` 升 patch，破坏性变更升 major。仅发布 PR 合并后创建不可变的 `vX.Y.Z` 标签，随后由现有打包 workflow 产出未签名安装包；不发布 crates.io 或 GitHub Release。

Cargo workspace 版本是唯一来源，发布分支同步 `Packager.toml`，普通 CI 校验它与 workspace 及内部 crate 的锁文件版本一致。首次发布以 `v0.1.0` 建立基线，不按历史提交重算版本。仓库需启用 GitHub Actions 创建 PR 的权限；失败后重跑 workflow，不自动重试递增版本或改写已有标签。
