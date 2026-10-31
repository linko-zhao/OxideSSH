# 16: 验证并开放 SSH Compression

**What to build:** 验证 russh 0.62.5 的 SSH 压缩协商能力；若支持，用户可为新连接启用压缩，若不支持则形成可核查的能力结论而不加入无效设置。

**Blocked by:** None (can start immediately).

**Status:** merged-tested

- [ ] 通过 russh 0.62.5 上游源码或文档确认压缩 API、协商方向和支持限制，并记录结论。
- [ ] 若支持，压缩设置可配置、持久化并具有中英文文案，新连接和重连实际按配置协商。
- [ ] 若支持，协商失败和远端不支持时的行为明确且可观察。
- [ ] 若不支持，不添加虚假的配置字段或 UI，并记录解除阻塞所需的上游条件。
- [ ] 任何新增配置均保持 `schemaVersion` 1 和旧配置兼容。

## Progress — 2026-10-25

- 实现提交 `131a8f2` 已合入 integration；russh 0.62.5 的 zlib 能力和 none 回退已核实。
- `cargo test -p oxide-ssh-core` 38 项、`cargo test -p oxide-ssh-desktop` 21 项通过，包含本地 SSH 压缩协商与数据传输；未对外部主机 smoke。
