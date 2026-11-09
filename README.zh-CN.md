<h1 align="center">
  <img src="assets/oxide-ssh-icon.svg" alt="OxideSSH 图标" width="56" height="56" align="absmiddle">
  OxideSSH
</h1>

<p align="center">
  面向 Windows 与 macOS 的原生、快速、内存安全的 SSH 客户端。
</p>

<p align="center">
  <a href="https://github.com/linko-zhao/OxideSSH/actions/workflows/ci.yml"><img src="https://github.com/linko-zhao/OxideSSH/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT 许可证">
  <img src="https://img.shields.io/badge/rust-1.97.1-orange.svg" alt="Rust 1.97.1">
</p>

<p align="center">
  <a href="README.md">English</a> | <b>简体中文</b>
</p>

OxideSSH 是用 Rust 编写的桌面 SSH 客户端，采用原生 GPU 渲染界面（[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)）。它管理保存的连接配置，在标签页中运行交互式 SSH 会话与本地 shell 会话，并将凭据保存在操作系统的凭据库中。

## 功能特性

### 连接与凭据
- **连接配置（Profile）** — 名称、主机、端口、用户名和认证方式；可在工作区中创建、编辑和删除。
- **认证方式** — 密码、带可选口令的私钥，或 SSH agent（OpenSSH agent；Windows 上回退到 Pageant）。
- **系统钥匙串存储** — 密码与私钥口令保存在 Windows 凭据管理器、macOS 钥匙串或 Linux Secret Service 中。磁盘上只持久化不透明的 `CredentialRef`；秘密从不写入磁盘或日志。
- **主机密钥校验** — 未知主机必须显式信任其 SHA-256 指纹；主机密钥变化时会阻断连接，必须手动处理。已信任主机可在设置中查看和删除。

### 工作区与标签页
- 可搜索的主机列表，支持彩色标签、多标签（OR）筛选、**无标签**筛选和**全部主机**视图。
- 每条 SSH 连接与每个本地终端各占一个标签页；标签条显示连接状态，关闭仍有活动会话的标签页会请求确认。
- **本地终端** — 在主目录中运行平台默认 shell（`$SHELL` 或 `/bin/sh`；Windows 按 PATH 探测 `pwsh.exe` → `powershell.exe`，回退 `%COMSPEC%`/`cmd.exe`）的标签页。

### 终端
- Terminal Engine 终端仿真，保留 10,000 行回滚历史，并提供后端无关的渲染接口。
- 支持终端程序的鼠标报告（点击、拖动和滚轮），按住 Shift 拖动仍用于本地文本选择；支持鼠标选择、复制/粘贴、bracketed paste、大段粘贴确认阈值配置、1 MiB 粘贴上限及 Ctrl/Alt/Shift 组合键。
- 支持 IME 文本输入、宽字符，终端尺寸变化会同步到远端会话。
- 主题（浅色 / 深色 / 跟随系统）与英文 / 简体中文界面。

### 桌面集成
- **设置** — 分类设置页面提供语言、新建标签页行为、大段粘贴确认阈值、外观和受信任主机，包含中英文文案。
- macOS：Cmd+Q 退出、Dock 图标，以及“新建窗口”菜单项。
- Windows 提供 `.msi` 安装包，macOS 提供 `.app` / `.dmg`。

> 侧边栏中的密钥链、端口转发、代码片段、已知主机和日志入口目前只是占位项。端口转发与代码片段尚未实现。

## 平台支持

| 平台 | 状态 | 产物 |
| --- | --- | --- |
| Windows x86_64 (MSVC) | 支持 | `.msi` 安装包 |
| macOS 14+（Apple Silicon） | 支持 | `.app`、`.dmg` |
| Linux | 仅开发环境 | 从源码构建（X11；`scripts/run-linux.sh` 面向 WSL2/WSLg） |

## 快速开始

### 前置条件
- Rust 1.97.1（通过 `rust-toolchain.toml` 自动安装）
- Zig 0.17.1（构建固定版本 libghostty-vt 1.0.0；Windows 使用 Git Bash 等 POSIX shell）
- Windows：MSVC 构建工具
- macOS：Xcode 命令行工具
- Linux：X11/xkb 开发库，以及用于“记住”凭据的 Secret Service 提供方（如 `gnome-keyring`）

### 构建与运行

先构建与当前 Rust 主机匹配的原生库。脚本会下载固定版本的源码及依赖归档，并在构建前验证两者的 SHA-256：

```sh
# 从以下目标中选择一个：
# x86_64-pc-windows-msvc | aarch64-pc-windows-msvc
# x86_64-apple-darwin | aarch64-apple-darwin
# x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu
target=x86_64-pc-windows-msvc
./scripts/build-libghostty-vt.sh "$target"
export LIBGHOSTTY_VT_LIB_DIR="$PWD/target/libghostty-vt/$target/stage/usr/lib"
cargo build -p oxide-ssh-desktop --locked
cargo run -p oxide-ssh-desktop --locked
```

脚本固定 libghostty-vt 1.0.0、Zig 0.17.1 与归档校验和，具体构建参数见 `scripts/build-libghostty-vt.sh`。

在 WSL2/WSLg 上使用启动脚本，它会补齐缺失的 GUI 库并启动钥匙串守护进程：

```bash
./scripts/run-linux.sh
```

`Makefile` 封装了常用任务：

```bash
make build        # 调试构建
make release      # 发布构建
make test         # 工作区测试
make lint         # rustfmt --check + clippy -D warnings
make package-win  # .msi（Windows 主机 / CI）
make package-mac  # .app + .dmg（macOS 主机 / CI）
```

## 键盘快捷键

macOS 使用 `Cmd`，Windows/Linux 使用 `Ctrl`（另有说明除外）。

| 操作 | macOS | Windows / Linux |
| --- | --- | --- |
| 新建连接 | `Cmd+N` | `Ctrl+N` |
| 新建本地终端 | `Cmd+Shift+N` | `Ctrl+Shift+N` |
| 设置 | `Cmd+,` | `Ctrl+,` |
| 关闭标签页 | `Cmd+W` | `Ctrl+Shift+W` |
| 复制终端选中内容 | `Cmd+C` | `Ctrl+Shift+C` |
| 粘贴到终端 | `Cmd+V` | `Ctrl+Shift+V` |
| 下一个 / 上一个标签页 | `Ctrl+Tab` / `Ctrl+Shift+Tab`，或 `Cmd+Shift+]` / `Cmd+Shift+[` | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| 退出 | `Cmd+Q` | — |

`Tab` 与 `Shift+Tab` 会发送给 shell；切换标签页请使用上表中的快捷键。

## 配置与数据

OxideSSH 使用操作系统标准的应用配置目录。典型位置如下（恢复界面会显示确切路径）：

| 平台 | 路径 |
| --- | --- |
| Windows | `%APPDATA%\linko-zhao\OxideSSH\config\` |
| macOS | `~/Library/Application Support/io.github.linko-zhao.OxideSSH/` |
| Linux | `~/.config/oxidessh/` |

| 文件 | 内容 |
| --- | --- |
| `config.json` | 连接配置、标签与应用设置 |
| `known_hosts.json` | 已信任的主机密钥与 SHA-256 指纹 |

写入采用原子方式（临时文件 + 重命名）。若存储根目录损坏，OxideSSH 会以只读恢复界面启动，绝不覆盖损坏的文件。

可在“设置 → 安全”导入或导出受信任主机。导入时可选择保留现有密钥或明确替换冲突密钥；损坏文件会被拒绝，且不会改变已存储的信任数据。

可在“设置 → 安全”导出配置、创建备份或恢复配置。导出文件不包含操作系统密钥环中的凭据或私钥文件内容；导入后需重新录入凭据。

## 项目结构

| Crate | 职责 |
| --- | --- |
| `oxide-ssh-core` | 连接配置、标签、设置、存储、凭据、主机密钥信任，以及 SSH 传输（`russh`） |
| `oxide-ssh-terminal` | 基于 Ghostty 的 Terminal Engine facade、输入编码、选择与后端无关的借用渲染快照 |
| `oxide-ssh-app` | 与 UI 工具包无关的标签页与会话编排 |
| `oxide-ssh-desktop` | GPUI 桌面应用：工作区、编辑器、终端视图、设置、国际化 |

## 参与贡献

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

提交信息请遵循 [Conventional Commits](https://www.conventionalcommits.org/)。项目术语定义见 `CONTEXT.md` 与 `GLOSSARY.md`。

## 许可证

MIT — 见 [LICENSE](LICENSE)。
