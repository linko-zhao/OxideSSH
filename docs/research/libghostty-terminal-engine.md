# libghostty 作为无 UI Terminal Engine 的可行性

**核查日期：2026-11-05**。范围限于 Ghostty 官方文档和 `ghostty-org/ghostty` 官方源码/讨论。本文把“libghostty”区分为 `libghostty-vt`（终端状态/VT 核心）与完整 GUI/嵌入式 surface 层。

## 结论

**可以只替换 terminal engine，不引入 Ghostty UI/runtime：应评估并集成 `libghostty-vt`，而不是完整 Ghostty surface。** 官方称 `libghostty-vt` 是从 Ghostty 提取的 C 库，运行时只需要该 C 库；其 API 支持独立创建终端、喂入 VT 字节、resize、查询终端状态/颜色、读取增量渲染网格，以及键鼠编码和选区/滚动等终端功能。[README](https://github.com/ghostty-org/ghostty#cross-platform-libghostty-for-embeddable-terminals) · [打包说明](https://github.com/ghostty-org/ghostty/blob/main/PACKAGING-LIBGHOSTTY.md) · [C API `terminal.h`](https://github.com/ghostty-org/ghostty/blob/main/include/ghostty/vt/terminal.h)

这不是 Ghostty GUI `Surface` 的无 UI 模式：使用 VT 库时，应用仍须自己提供 PTY 生命周期、事件循环、按键/鼠标事件来源、渲染、字体与 UI；Ghostty 提供的是状态与低层 API，不是 OxideSSH 的 GPUI 终端组件。**API 仍在开发中且未稳定**：官方文档明确警告 API 会变化；发布打包文档指出 0.x 次版本可破坏兼容性，需固定版本并准备重建/适配。[libghostty-vt API 文档](https://libghostty.tip.ghostty.org/) · [版本/ABI 规则](https://github.com/ghostty-org/ghostty/blob/main/PACKAGING-LIBGHOSTTY.md#versioning-and-abi)

## 能力核对

| 需求 | 公开状态 | 证据与边界 |
| --- | --- | --- |
| 独立创建终端 | **有** | `ghostty_terminal_new(allocator, &terminal, cols, rows)` 创建终端实例，free 时独立释放；不是 GUI surface。[terminal API](https://libghostty.tip.ghostty.org/terminal_8h.html) |
| 输入 PTY 输出字节 | **有** | `ghostty_terminal_vt_write` 与 `ghostty_terminal_vt_write_until_ground` 接收 VT 字节流。PTY 的读写和线程由宿主负责；终端发出的查询响应等需宿主配置 `WRITE_PTY` 回调并回写 PTY。[terminal API](https://libghostty.tip.ghostty.org/terminal_8h.html) · [`terminal.h` effects](https://github.com/ghostty-org/ghostty/blob/main/include/ghostty/vt/terminal.h) |
| resize | **有** | `ghostty_terminal_resize` 接收列、行及 cell 像素尺寸；PTY 的窗口大小更新仍需宿主同步处理。[terminal API](https://libghostty.tip.ghostty.org/terminal_8h.html) |
| 读取网格和样式 | **有** | `libghostty-vt` 的 render-state API 面向自定义 renderer，支持读取 row/cell/style、cursor、selection 等，并带 dirty tracking；终端 getter 另可读取列行、光标、活动 screen、颜色和模式。它提供渲染数据，不负责绘制。[Render State](https://libghostty.tip.ghostty.org/group__render.html) · [Terminal API](https://libghostty.tip.ghostty.org/terminal_8h.html) |
| 键盘输入 | **有编码器，不是完整 UI 输入处理** | `ghostty_key_encoder` 编码键事件为终端 escape bytes，能从终端同步应用模式；宿主负责把应用输入事件映射为 Ghostty key event，并把编码结果写入 PTY。[Key Encoding](https://libghostty.tip.ghostty.org/group__key.html) |
| 鼠标输入 | **有编码器，不是完整 UI 输入处理** | `ghostty_mouse_encoder` 编码鼠标事件，且能依据终端状态同步 tracking mode；宿主提供事件、坐标/尺寸并写回 PTY。[Mouse Support](https://libghostty.tip.ghostty.org/group__mouse.html) |
| 滚动 | **有终端 viewport 滚动能力** | `ghostty_terminal_scroll_viewport` 接收顶部、底部、delta、行等滚动行为；滚动条/scrollback 状态可查询。宿主仍实现滚动条、手势和呈现。[Terminal API](https://libghostty.tip.ghostty.org/terminal_8h.html) |
| 选区 | **有底层选择逻辑/数据** | 公开 selection API 提供 grid refs、选择快照、鼠标手势状态机、选词/选行/选输出/全选和格式化；render state 可呈现选中状态。宿主实现指针交互、复制/剪贴板集成与视觉反馈。[Selection](https://libghostty.tip.ghostty.org/group__selection.html) · [Render State](https://libghostty.tip.ghostty.org/group__render.html) |
| Ghostty surface/UI/runtime 可否完全剥离 | **可剥离的路线是 `libghostty-vt`；完整 surface 不等同于 headless engine** | 官方将 `libghostty-vt` 单独拆分，说明其只需 C library 运行；`Surface.zig` 将 surface 定义为绘制终端并响应事件的 widget。未找到“完整 Ghostty surface 可无 UI/runtime 独立作 terminal engine”的官方承诺。[README](https://github.com/ghostty-org/ghostty#cross-platform-libghostty-for-embeddable-terminals) · [`Surface.zig`](https://github.com/ghostty-org/ghostty/blob/main/src/Surface.zig) |

## 构建、链接与 Rust FFI

- 官方提供独立 `libghostty-vt` 发布源 tarball（版本号独立于 Ghostty app），且 Windows 是支持目标；安装包括 Windows DLL、`ghostty-vt.lib` 导入库、`ghostty-vt-static.lib` 和 C headers。可动态或静态链接；需在应用分发中管理 DLL 或静态链接依赖。[打包说明：源包/安装文件](https://github.com/ghostty-org/ghostty/blob/main/PACKAGING-LIBGHOSTTY.md#source-tarballs) · [安装文件](https://github.com/ghostty-org/ghostty/blob/main/PACKAGING-LIBGHOSTTY.md#installed-files)
- 从源码构建需要官方指定版本 Zig；主仓库源码 checkout 要显式 `-Demit-lib-vt`，否则默认构建 Ghostty app。发布 tarball、Zig build 选项及 CMake 集成在官方说明中列出；官方 CMake wrapper 本身仍要求 PATH 上有 Zig。[打包/构建说明](https://github.com/ghostty-org/ghostty/blob/main/PACKAGING-LIBGHOSTTY.md#building-libghostty-vt) · [CMake 指南](https://github.com/ghostty-org/ghostty/blob/main/dist/cmake/README.md)
- API 是 **C ABI**（`<ghostty/vt.h>` 等公开 headers），因此 Rust 可以通过 FFI 调用该 C ABI；但在核查的官方资料中**未找到官方 Rust crate/bindings 或 Rust API 支持承诺**。需要本项目维护 bindings（例如手写 `extern "C"`/bindgen）并处理 opaque handles、回调、所有权/生命周期及版本匹配。这部分是基于 C API 的集成方式，不代表官方 Rust 绑定。[公开 C headers](https://github.com/ghostty-org/ghostty/tree/main/include/ghostty) · [官方 README 的 C/Zig 定位](https://github.com/ghostty-org/ghostty#cross-platform-libghostty-for-embeddable-terminals)
- Zig/C API 的行为和终端核心已经用于 Ghostty，但官方明确区分了核心成熟度与 API 稳定性；不能把它当稳定 Rust 依赖或假设不同 0.x 次版本 ABI/API 兼容。[README](https://github.com/ghostty-org/ghostty#cross-platform-libghostty-for-embeddable-terminals) · [版本/ABI 规则](https://github.com/ghostty-org/ghostty/blob/main/PACKAGING-LIBGHOSTTY.md#versioning-and-abi)

## 决策摘要

对于“保留 OxideSSH 的 GPUI/PTTY/UI，只替换 Alacritty terminal engine”这一目标，**公开接口层面具备可行性，目标应是 `libghostty-vt`**。它覆盖终端模拟、状态读取与 renderer/输入所需的底层积木，但不是可直接塞进现有视图的现成渲染控件；主要工程成本/风险在自有 renderer 与宿主输入、PTY effects/回写、C ABI 绑定，以及 API 尚未稳定。没有证据表明必须引入完整 Ghostty GUI 或其 UI runtime；也没有官方 Rust 绑定证据。
