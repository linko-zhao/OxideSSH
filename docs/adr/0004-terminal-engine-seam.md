# 以 Terminal Engine facade 隔离终端后端

**Status**: accepted

`oxide-ssh-terminal` exposes a concrete `TerminalEngine` facade and backend-neutral borrowed snapshots; app and desktop consume terminal semantics only. The accepted design excludes runtime backend dispatch and the Ghostty UI runtime. TerminalEngine is backed exclusively by pinned libghostty-vt, whose C ABI and opaque handles remain internal.

## Considered Options

- **公共 trait + 命令/事件入口**：被拒绝。当前没有运行时多后端需求，统一命令 enum 和可选能力错误会降低调用点可读性，并增加未使用的 interface。
- **直接封装并公开 Ghostty C 类型**：被拒绝。app、desktop 与测试只依赖终端语义，FFI 所有权及 ABI 留在引擎内部。

## Consequences

- `TerminalEngine` is the single application-facing backend contract; backend-specific modes, grids, damage and cursor types stay private.
- Input encoding reads terminal mode inside the engine; renderers consume backend-neutral cells, styles, selection and cursor state.
- Contract tests observe behavior through the public interface and do not inspect backend internals.
- The libghostty-vt C ABI must remain inside an internal adapter, with opaque handles and lifecycle managed there.

## Implementation status

TerminalEngine now uses libghostty-vt only. The Rust facade retains backend-neutral sizing, input, effect, selection, color and snapshot APIs. Build the pinned 1.0.0 source and matching dependency archive with Zig 0.17.1 using `scripts/build-libghostty-vt.sh`; the script validates both archive SHA-256 checksums before compilation.
