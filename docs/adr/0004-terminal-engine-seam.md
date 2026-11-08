# 以 Terminal Engine facade 隔离终端后端

**Status**: accepted

`oxide-ssh-terminal` 以具体的 `TerminalEngine` facade 和后端无关的借用快照作为公开 interface，封闭 Alacritty 的模式、网格、damage 与游标类型；app 和 desktop 只消费终端语义。后端在编译时选择，当前仅迁移现有 Alacritty 实现并保持行为契约，未来以内部 adapter 接入 libghostty-vt，不引入运行时动态分发或 Ghostty UI runtime。

## Considered Options

- **公共 trait + 命令/事件入口**：被拒绝。当前没有运行时多后端需求，统一命令 enum 和可选能力错误会降低调用点可读性，并增加未使用的 interface。
- **直接封装并公开 Alacritty 类型**：被拒绝。app、desktop 与测试仍依赖后端网格和模式，无法通过替换内部 adapter 完成迁移。

## Consequences

- `TerminalModel` 干净迁移为 `TerminalEngine`，不保留旧公开路径。
- 输入编码在引擎内部读取终端模式；renderer 通过后端无关快照读取单元、样式、选区和游标。
- 契约测试只通过公开 interface 观察行为，不读取 Alacritty 内部网格。
- 本期不实现 libghostty-vt FFI；未来接入时需固定 C API 版本并维护 Rust 绑定。

## 后续实现状态（Ghostty ticket #5）

`ghostty-vt` feature 已加入私有的 libghostty-vt render-state adapter，可生成现有的后端无关 `TerminalSnapshot`。`TerminalEngine` 仍由 Alacritty 实现；此快照接入不改变应用运行时，也不公开 Ghostty FFI 类型。
